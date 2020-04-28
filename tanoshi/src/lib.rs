use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MangaModel {
    pub title: String,
    pub author: String,
    //pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub path: String,
    pub thumbnail_url: String,
    pub last_read: Option<String>,
    pub last_page: Option<i32>,
    pub is_favorite: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetMangasResponse {
    pub mangas: Vec<MangaModel>,
}