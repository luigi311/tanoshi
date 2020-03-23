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
            "INSERT INTO history(user_id, chapter_id, last_page, at)
        VALUES(
        (SELECT id FROM user WHERE username = ?1),
        (SELECT chapter.id FROM chapter
        JOIN manga ON manga.id = chapter.manga_id
        JOIN source ON source.id = manga.source_id
        WHERE source.name = ?2 AND manga.title = ?3 AND chapter.number = ?4),
        ?5, ?6)",
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
        HistoryResponse {
            history: vec![],
            status: "success".to_string(),
        }
    }
}
