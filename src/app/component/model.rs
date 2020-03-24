use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MangaModel {
    pub title: String,
    pub author: String,
    pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub path: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ChapterModel {
    pub no: String,
    pub url: String,
    pub read: i32,
    pub uploaded: Option<chrono::DateTime<Local>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetMangasResponse {
    pub mangas: Vec<MangaModel>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetMangaResponse {
    pub manga: MangaModel,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetChaptersResponse {
    pub chapters: Vec<ChapterModel>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetPagesResponse {
    pub pages: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetFavoritesResponse {
    pub favorites: Option<Vec<FavoriteManga>>,
    pub status: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AddFavoritesResponse {
    pub status: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct FavoriteManga {
    pub source: String,
    pub title: String,
    pub path: String,
    pub thumbnail_url: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub enum ReadingDirection {
    LeftToRight,
    RightToLeft,
    LongStrip,
}

#[derive(Deserialize, Serialize, Clone)]
pub enum BackgroundColor {
    Black,
    White,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Settings {
    pub double_page: bool,
    pub background_color: BackgroundColor,
    pub reading_direction: ReadingDirection,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            double_page: false,
            background_color: BackgroundColor::Black,
            reading_direction: ReadingDirection::LeftToRight,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryRequest {
    pub source: String,
    pub title: String,
    pub chapter: String,
    pub read: i32,
    pub at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryResponse {
    pub history: Vec<ChapterModel>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateModel {
    pub source: String,
    pub title: String,
    pub thumbnail_url: String,
    pub number: String,
    pub uploaded: chrono::DateTime<Local>,
    pub days: Option<i64>,
    pub show_sep: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatesResponse {
    pub updates: Vec<UpdateModel>,
    pub status: String,
}
