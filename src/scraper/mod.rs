use chrono::Local;
use serde::{Deserialize, Serialize};

pub mod mangasee;
pub mod repository;
use mangasee::Mangasee;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Manga {
    pub title: String,
    pub author: String,
    pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub path: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Chapter {
    pub no: String,
    pub url: String,
    pub read: i32,
    pub uploaded: chrono::DateTime<Local>,
}

impl Default for Chapter {
    fn default() -> Self {
        Chapter {
            no: "".to_string(),
            url: "".to_string(),
            read: 0,
            uploaded: Local::now(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Params {
    pub keyword: Option<String>,
    pub page: Option<String>,
    pub sort_order: Option<String>,
    pub sort_by: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetParams {
    pub refresh: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetMangasResponse {
    pub mangas: Vec<Manga>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetMangaResponse {
    pub manga: Manga,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetChaptersResponse {
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetPagesResponse {
    pub pages: Vec<String>,
}

pub trait Scraping {
    fn get_mangas(url: &String, param: Params) -> GetMangasResponse;
    fn get_manga_info(url: &String) -> GetMangaResponse;
    fn get_chapters(url: &String) -> GetChaptersResponse;
    fn get_pages(url: &String) -> GetPagesResponse;
}
