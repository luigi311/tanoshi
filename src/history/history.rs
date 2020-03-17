use sled::Db;

use crate::history::{HistoryRequest, HistoryResponse};
use crate::model::{Chapter, Manga};
use std::collections::BTreeMap;

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
        source: String,
        title: String,
        chapter: HistoryRequest,
        db: Db,
    ) -> HistoryResponse {
        let mut history = db.fetch_and_update(
            format!("history#{}#{}#{}", username, source, title),
            |fav: Option<&[u8]>| {
                let mut history: Vec<Chapter> = match fav {
                    Some(bytes) => serde_json::from_slice(bytes).unwrap(),
                    None => vec![],
                };

                match history.iter().position(|ch| ch.path == chapter.path) {
                    Some(idx) => {
                        history[idx].read = chapter.read;
                        history[idx].len = chapter.len;
                    }
                    None => {
                        history.push(Chapter {
                            path: chapter.path.clone(),
                            read: chapter.read,
                            len: chapter.len,
                        });
                    }
                }

                serde_json::to_vec(&history).ok()
            },
        );

        match history {
            Ok(_) => HistoryResponse {
                history: vec![],
                status: "success".to_string(),
            },
            Err(e) => HistoryResponse {
                history: vec![],
                status: format!("failed set history, reason: {}", e.to_string()),
            },
        }
    }

    pub fn get_history(
        &self,
        username: String,
        source: String,
        title: String,
        db: Db,
    ) -> HistoryResponse {
        let key = format!("history#{}#{}#{}", username, source, title);
        let history: Vec<Chapter> = match db.get(&key).unwrap() {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap(),
            None => vec![],
        };

        HistoryResponse {
            history,
            status: "success".to_string(),
        }
    }
}
