use crate::model::{Document, Manga};
use serde::{Deserialize, Serialize};

pub mod favorites;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetFavoritesResponse {
    pub favorites: Vec<Manga>,
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
}
