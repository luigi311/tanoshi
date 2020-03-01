use serde::{Deserialize, Serialize};
use sled::Db;

pub mod mangasee;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manga {
    pub title: String,
    pub author: String,
    pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub path: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Chapter {
    pub no: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Params {
    pub keyword: Option<String>,
    pub page: Option<String>,
    pub sort_order: Option<String>,
    pub sort_by: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetMangasResponse {
    mangas: Vec<Manga>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetMangaResponse {
    manga: Manga,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetChaptersResponse {
    chapters: Vec<Chapter>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetPagesResponse {
    pages: Vec<String>,
}

pub trait Scraping {
    fn get_mangas(&self, param: Params) -> GetMangasResponse;
    fn get_latest_mangas(&self) -> GetMangasResponse;
    fn get_manga_info(&self, path: String, db: Db) -> GetMangaResponse;
    fn get_chapters(&self, path: String, db: Db) -> GetChaptersResponse;
    fn get_pages(&self, path: String, db: Db) -> GetPagesResponse;
}
