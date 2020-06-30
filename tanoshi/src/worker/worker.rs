use crate::bot::TextType;
use crate::extension::Extensions;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::{mpsc::Sender, Arc, RwLock};
use std::time::Duration;
use tanoshi_lib::extensions::Extension;
use tbot::types::chat::Id;
use tokio::runtime::{self, Runtime};
use tokio::time::delay_for;

#[derive(Debug, Clone)]
pub struct MangaUserUpdate {
    source: String,
    manga_id: i64,
    manga_title: String,
    url: String,
    users: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    id: i64,
    number: String,
}

#[derive(Debug, Clone)]
pub struct MangaChapter {
    title: String,
    chapters: Vec<Chapter>,
}

pub struct Worker {
    pub rt: Runtime,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            rt: runtime::Builder::new()
                .threaded_scheduler()
                .enable_all()
                .build()
                .unwrap(),
        }
    }

    pub fn start_interval<F>(&self, interval: u64, f: F)
    where
        F: FnOnce() + Send + Clone + 'static,
    {
        self.rt.spawn(async move {
            loop {
                let f_clone = f.clone();
                f_clone();
                delay_for(Duration::from_secs(interval)).await;
            }
        });
    }

    pub fn remove_cache(&self, interval: u64) {
        if interval == 0 {
            return;
        }

        self.start_interval(interval * 84600, move || {
            if let Some(cache_dir) = dirs::home_dir() {
                let cache_dir = cache_dir.join(".tanoshi").join("cache");
                match std::fs::remove_dir_all(cache_dir) {
                    Ok(_) => {}
                    Err(e) => error!("error remove cache: {}", e),
                }
            }
        });
    }

    pub fn check_update(
        &self,
        interval: u64,
        database_path: String,
        base_url: String,
        exts: Arc<RwLock<Extensions>>,
        bot_pub: Option<Sender<(Id, String, TextType)>>,
    ) {
        if interval == 0 {
            return;
        }

        self.start_interval(interval * 3600, move || {
            info!("Updating chapters....");
            match Connection::open(database_path.clone()) {
                Ok(mut conn) => {
                    let mut res: Vec<MangaUserUpdate> = vec![];
                    {
                        let mut stmt = match conn.prepare(
                            "SELECT s.name, m.id, m.title, s.url || m.path AS url, GROUP_CONCAT(user_id, ',') AS users
                              FROM favorite
                              JOIN manga m on favorite.manga_id = m.id
                              JOIN source s on m.source_id = s.id
                              GROUP BY manga_id;")
                        {
                            Ok(stmt) => stmt,
                            Err(e) => {
                                error!("error prepare statement: {}", e);
                                return
                            }
                        };
                        res = stmt
                            .query_map(params![], |row| {
                                let user_ids: String = row.get(4)?;
                                let users = user_ids
                                    .split(",")
                                    .map(|u| u.parse::<i64>().unwrap())
                                    .collect::<Vec<i64>>();
                                Ok(MangaUserUpdate {
                                    source: row.get(0)?,
                                    manga_id: row.get(1)?,
                                    manga_title: row.get(2)?,
                                    url: row.get(3)?,
                                    users,
                                })
                            })
                            .unwrap()
                            .filter_map(|m| m.ok())
                            .collect();
                    }

                    let mut chapter_updates = HashMap::new();
                    {
                        let tx = conn.transaction().unwrap();
                        for m in res {
                            let exts = exts.read().unwrap();
                            let chapters = if let Some(ext) = exts.get(&m.source) {
                                if let Ok(chapters) = ext.get_chapters(&m.url) {
                                    chapters
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            };

                            for c in chapters {
                                for u in m.clone().users {
                                    if let Err(e) = tx.execute(
                                        r#"INSERT INTO chapter(user_id, manga_id, number, title, path, uploaded)
                                            VALUES(
                                            ?1,
                                            ?2,
                                            ?3,
                                            ?4,
                                            ?5,
                                            ?6) ON CONFLICT DO NOTHING"#,
                                        params![u, m.manga_id, c.no.clone(), c.title, c.url, c.uploaded],
                                    ) {
                                        error!("erorr insert row: {}", e);
                                    }
                                    let last_id = tx.last_insert_rowid();
                                    if last_id > 0 {
                                        let user_val: &mut HashMap<String, Vec<Chapter>> = chapter_updates.entry(u).or_insert(HashMap::new());
                                        let manga_val: &mut Vec<Chapter> = user_val.entry(m.manga_title.clone()).or_insert(vec![]);
                                        if manga_val.iter().find(|u| u.id == last_id).is_none() {
                                            manga_val.push(Chapter { id: last_id, number: c.no.clone() });
                                        }
                                    }
                                }
                            }
                        }
                        if let Err(e) = tx.commit() {
                            error!("erorr commit: {}", e);
                        }
                    }
                    {
                        if let Some(tx) = bot_pub {
                            for (user_id, manga_map) in chapter_updates {
                                if let Ok(telegram_chat_id) = conn.query_row(r#"SELECT telegram_chat_id FROM "user" WHERE id = ?1"#, params![user_id], |row| row.get::<_, i64>(0)) {
                                    for (title, chapters) in manga_map {
                                        let mut text = format!("<b>{}</b>\n", title);
                                        for ch in chapters.clone() {
                                            text += format!("<a href=\"{}/chapter/{}/page/1\">Chapter {}</a>\n", base_url.clone(), ch.id, ch.number).as_str();
                                        }
                                        if let Err(e) = tx.send((tbot::types::chat::Id(telegram_chat_id), text, TextType::HTML)) {
                                            error!("error send update: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("error update chapter: {}", e),
            }
            info!("Updating chapters done");
        });
    }
}
