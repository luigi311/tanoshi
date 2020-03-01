use sled::Db;

use crate::history::{HistoryChapter, HistoryManga, HistoryResponse};

#[derive(Clone)]
pub struct History {
    db: Db,
}

impl Default for History {
    fn default() -> Self {
        History {
            db: sled::open("./db/history").unwrap(),
        }
    }
}

impl History {
    pub fn add_history(
        &self,
        username: String,
        source: String,
        title: String,
        chapter: HistoryChapter,
    ) -> HistoryResponse {
        let key = format!("history:{}:{}:{}:", username, source, title);
        let history = self.db.fetch_and_update(key, |fav: Option<&[u8]>| {
            let mut history: Vec<HistoryChapter> = match fav {
                Some(bytes) => {
                    let history: Vec<HistoryChapter> = serde_json::from_slice(bytes).unwrap();
                    history
                }
                None => vec![],
            };
            match history.iter().position(|ch| ch.path == chapter.path) {
                Some(idx) => history[idx].read = chapter.read,
                None => history.push(chapter.clone()),
            }

            serde_json::to_vec(&history).ok()
        });

        match history.unwrap() {
            Some(bytes) => HistoryResponse {
                manga: Some(HistoryManga { source, title }),
                chapters: serde_json::from_slice(&bytes).ok(),
                status: "success".to_string(),
            },
            None => HistoryResponse {
                manga: None,
                chapters: None,
                status: "failed".to_string(),
            },
        }
    }

    pub fn get_history(&self, username: String, source: String, title: String) -> HistoryResponse {
        let key = format!("history:{}:{}:{}:", username, source, title);
        let history = match self.db.get(&key).unwrap() {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap(),
            None => None,
        };
        HistoryResponse {
            manga: Some(HistoryManga { source, title }),
            chapters: history,
            status: "success".to_string(),
        }
    }
}
