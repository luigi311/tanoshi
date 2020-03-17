use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod history;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryRequest {
    chapter: Option<String>,
    read: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryResponse {
    pub source: String,
    pub title: String,
    history: Option<BTreeMap<String, i32>>,
    pub(crate) status: String,
}
