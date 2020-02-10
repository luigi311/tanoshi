use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod app;
pub use app::{App, AppRoute};

pub mod component;
pub mod catalogue;
pub mod detail;
pub mod chapter;
pub mod home;

#[derive(Deserialize, Debug)]
pub struct MangaModel {
    pub title: String,
    pub author: String,
    pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub url: String,
    pub thumbnail_url: String,
    pub chapter: Vec<ChapterModel>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChapterModel {
    pub chapter: String,
    pub url: String,
    pub pages: Vec<String>,
}