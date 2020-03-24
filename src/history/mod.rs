use crate::model::{Chapter, Manga};
use serde::{Deserialize, Serialize};

pub mod history;
use history::History;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryRequest {
    source: String,
    title: String,
    chapter: String,
    read: i32,
    at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryResponse {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<Chapter>,
    pub status: String,
}
