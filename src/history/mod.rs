use serde::{Deserialize, Serialize};
pub mod history;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryManga {
    pub source: String,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryChapter {
    path: String,
    read: usize,
    len: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryResponse {
    manga: Option<HistoryManga>,
    chapters: Option<Vec<HistoryChapter>>,
    pub(crate) status: String,
}
