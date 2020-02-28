use serde::{Deserialize, Serialize};

pub mod app;
pub use app::{App, AppRoute};

pub mod component;
pub mod catalogue;
pub mod detail;
pub mod chapter;
pub mod home;

#[derive(Deserialize, Debug, Clone)]
pub struct MangaModel {
    pub title: String,
    pub author: String,
    pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub path: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ChapterModel {
    pub no: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetMangasResponse {
    mangas: Vec<MangaModel>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetMangaResponse {
    manga: MangaModel,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetChaptersResponse {
    chapters: Vec<ChapterModel>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetPagesResponse {
    pages: Vec<String>,
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

#[derive(Debug, Deserialize, Clone)]
pub struct FavoriteManga {
    pub source: String,
    pub title: String,
    pub path: String,
    pub thumbnail_url: String,
}