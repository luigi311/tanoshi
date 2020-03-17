use sled::Db;

use crate::history::{HistoryRequest, HistoryResponse};
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
        let key = format!("history:{}:{}:{}:", username, source, title);
        let history = db.fetch_and_update(key, |fav: Option<&[u8]>| {
            let mut history: BTreeMap<String, i32> = match fav {
                Some(bytes) => {
                    let history: BTreeMap<String, i32> = serde_json::from_slice(bytes).unwrap();
                    history
                }
                None => BTreeMap::default(),
            };

            history.insert(
                chapter.chapter.as_ref().unwrap().to_string(),
                *chapter.read.as_ref().unwrap(),
            );
            serde_json::to_vec(&history).ok()
        });

        match history.unwrap() {
            Some(bytes) => HistoryResponse {
                source,
                title,
                history: serde_json::from_slice(&bytes).ok(),
                status: "success".to_string(),
            },
            None => HistoryResponse {
                source,
                title,
                history: Some(BTreeMap::default()),
                status: "failed".to_string(),
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
        let key = format!("history:{}:{}:{}:", username, source, title);
        let history = match db.get(&key).unwrap() {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap(),
            None => None,
        };
        HistoryResponse {
            source,
            title,
            history,
            status: "success".to_string(),
        }
    }
}
