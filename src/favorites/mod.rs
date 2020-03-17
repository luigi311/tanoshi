use crate::model::Manga;
use serde::{Deserialize, Serialize};

pub mod favorites;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetFavoritesResponse {
    pub favorites: Vec<FavoriteManga>,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AddFavoritesResponse {
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FavoriteManga {
    pub source: String,
    pub title: String,
    pub path: String,
    pub thumbnail_url: String,
}
