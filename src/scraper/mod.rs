use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod mangasee;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manga {
    pub title: String,
    pub author: String,
    pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub url: String,
    pub thumbnail_url: String,
    pub chapter: HashMap<String, Chapter>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Chapter {
    pub url: String,
    pub pages: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Params {
    pub keyword: Option<String>,
    pub page: Option<String>,
    pub sort_order: Option<String>,
    pub sort_by: Option<String>,
}

pub trait Scraping {
    fn new(url: &'static str) -> Self;
    fn get_mangas(&self, param: Params) -> Vec<Manga>;
    fn get_latest_mangas(&self) -> Vec<Manga>;
    fn get_manga_info(&self, manga: &Manga) -> Manga;
    fn get_chapter(&self, chapter: &mut Chapter);
}
