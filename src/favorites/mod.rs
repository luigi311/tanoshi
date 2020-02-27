use crate::scraper::{Chapter, Manga};
use serde::{Deserialize, Serialize};
pub mod favorites;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FavoritesResponse {
    pub favorites: Option<Vec<Manga>>,
    pub status: String,
}
