use crate::model::{Chapter, Manga};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod history;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryRequest {
    source: String,
    title: String,
    chapter: String,
    read: i32,
    len: i32,
    at: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryResponse {
    history: Vec<Chapter>,
    pub status: String,
}
