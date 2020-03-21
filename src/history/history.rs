use sled::Tree;

use crate::history::{HistoryRequest, HistoryResponse};
use crate::model::{Chapter, Document, History as HistoryModel, Manga};
use crate::scraper::mangasee::Mangasee;
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
        request: HistoryRequest,
        library_tree: Tree,
        scraper_tree: Tree,
    ) -> HistoryResponse {
        let chapter = match scraper_tree
            .get(format!(
                "{}:{}:{}",
                request.source.clone(),
                request.title.clone(),
                request.chapter.clone()
            ))
            .unwrap()
        {
            Some(ret) => Chapter {
                path: String::from_utf8(ret.to_vec()).unwrap_or("".to_string()),
                read: request.read,
                len: request.len,
            },
            None => {
                return HistoryResponse {
                    history: vec![],
                    status: "Chapter not found".to_string(),
                }
            }
        };

        let history = HistoryModel {
            path: chapter.path.clone(),
            timestamp: request.at,
        };

        let key = format!(
            "{}:{}:{}",
            username,
            request.source.clone(),
            request.title.clone()
        );

        library_tree
            .merge(&key, serde_json::to_vec(&chapter).unwrap())
            .unwrap();
        library_tree
            .merge(&key, serde_json::to_vec(&history).unwrap())
            .unwrap();

        HistoryResponse {
            history: vec![],
            status: "success".to_string(),
        }
    }

    pub fn get_history(
        &self,
        username: String,
        source: String,
        title: String,
        library_tree: Tree,
    ) -> HistoryResponse {
        let key = format!("{}:{}:{}", username, source, title);
        let doc: Document = match library_tree.get(&key).unwrap() {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap(),
            None => Document::default(),
        };

        HistoryResponse {
            history: doc.chapters.clone(),
            status: "success".to_string(),
        }
    }
}
