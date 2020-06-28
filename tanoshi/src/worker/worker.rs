use crate::extension::manga::Manga;
use crate::extension::Extensions;
use rusqlite::{params, Connection};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tanoshi_lib::extensions::Extension;
use tokio::runtime::{self, Runtime};
use tokio::time::delay_for;

#[derive(Debug, Clone)]
pub struct MangaUserUpdate {
    source: String,
    manga_id: i64,
    url: String,
    users: Vec<i64>,
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
                let f = f.clone();
                f();
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
                let cache_dir = cache_dir.join(".tanoshi/cache");
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
        exts: Arc<RwLock<Extensions>>,
    ) {
        if interval == 0 {
            return;
        }

        self.start_interval(interval * 3600, move || {
            match Connection::open(database_path.clone()) {
                Ok(mut conn) => {
                    let mut res: Vec<MangaUserUpdate> = vec![];
                    {
                        let mut stmt = conn.prepare(
                            "SELECT s.name, m.id, s.url || m.path AS url, GROUP_CONCAT(user_id, ',') AS users
                              FROM favorite
                              JOIN manga m on favorite.manga_id = m.id
                              JOIN source s on m.source_id = s.id
                              GROUP BY manga_id;")
                            .unwrap();
                        res = stmt
                            .query_map(params![], |row| {
                                let user_ids: String = row.get(3)?;
                                let users = user_ids
                                    .split(",")
                                    .map(|u| u.parse::<i64>().unwrap())
                                    .collect::<Vec<i64>>();
                                Ok(MangaUserUpdate {
                                    source: row.get(0)?,
                                    manga_id: row.get(1)?,
                                    url: row.get(2)?,
                                    users,
                                })
                            })
                            .unwrap()
                            .filter_map(|m| m.ok())
                            .collect();
                    }
                    {
                        let tx = conn.transaction().unwrap();
                        let mut chapter_updates = HashMap::new();
                        for m in res {
                            let exts = exts.read().unwrap();
                            let chapters = exts.get(&m.source).unwrap().get_chapters(&m.url).unwrap();
                            for c in chapters {
                                for u in m.clone().users {
                                    tx.execute(
                                        r#"INSERT INTO chapter(user_id, manga_id, number, title, path, uploaded)
                                            VALUES(
                                            ?1,
                                            ?2,
                                            ?3,
                                            ?4,
                                            ?5,
                                            ?6) ON CONFLICT DO NOTHING"#,
                                        params![u, m.manga_id, c.no, c.title, c.url, c.uploaded],
                                    ).unwrap();
                                    let last_id = tx.last_insert_rowid();
                                    if last_id > 0 {
                                        let val: &mut Vec<i64> = chapter_updates.entry(u).or_insert(vec![]).as_mut();
                                        if !val.contains(&last_id) {
                                            val.push(last_id);
                                        }
                                    }
                                }
                            }
                        }
                        tx.commit().unwrap();
                    }
                }
                Err(e) => error!("error update chapter: {}", e),
            }
        });
    }
}
