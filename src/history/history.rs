use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use crate::history::{HistoryRequest, HistoryResponse};
use crate::model::{Chapter, Document, History as HistoryModel, Manga};
use crate::scraper::mangasee::Mangasee;

#[derive(Clone)]
pub struct History {}

impl Default for History {
    fn default() -> Self {
        History {}
    }
}

impl History {
    pub fn add_history(
        &self,
        username: String,
        request: HistoryRequest,
        db: Arc<Mutex<Connection>>,
    ) -> HistoryResponse {
        let conn = db.lock().unwrap();
        match conn.execute(
            "INSERT INTO history(user_id, chapter_id, last_page, at) \
        VALUES(\
        (SELECT id FROM user WHERE username = ?1), \
        (SELECT chapter.id FROM chapter \
        JOIN manga ON manga.id = chapter.manga_id \
        JOIN source ON source.id = manga.source_id \
        WHERE source.name = ?2 \
        AND manga.title = ?3 \
        AND chapter.number = ?4), \
        ?5, \
        ?6) \
        ON CONFLICT(user_id, chapter_id) \
         DO UPDATE SET last_page = excluded.last_page, \
         at = excluded.at, \
         updated = CURRENT_TIMESTAMP",
            params![
                username,
                request.source,
                request.title,
                request.chapter,
                request.read,
                request.at
            ],
        ) {
            Ok(_) => HistoryResponse {
                history: vec![],
                status: "success".to_string(),
            },
            Err(e) => HistoryResponse {
                history: vec![],
                status: format!("failed, reason: {}", e.to_string()),
            },
        }
    }

    pub fn get_history(
        &self,
        username: String,
        source: String,
        title: String,
        db: Arc<Mutex<Connection>>,
    ) -> HistoryResponse {
        let conn = db.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT chapter.number, history.last_page FROM history
            JOIN user ON user.id = history.user_id
            JOIN chapter ON chapter.id = history.chapter_id
            JOIN manga ON manga.id = chapter.manga_id
            JOIN source ON source.id = manga.source_id
            WHERE user.username = ?1 AND source.name = ?2 AND manga.title = ?3",
            )
            .unwrap();
        let history_iter = stmt
            .query_map(params![username, source, title], |row| {
                Ok(Chapter {
                    path: row.get(0)?,
                    read: row.get(1)?,
                })
            })
            .unwrap();

        let mut histories = vec![];
        for ch in history_iter {
            histories.push(ch.unwrap())
        }

        HistoryResponse {
            history: histories,
            status: "success".to_string(),
        }
    }
}
