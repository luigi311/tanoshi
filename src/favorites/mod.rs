use serde::{Deserialize, Serialize};
pub mod favorites;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetFavoritesResponse {
    pub favorites: Option<Vec<FavoriteManga>>,
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

impl PartialEq for FavoriteManga {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.title == other.title
            && self.path == other.path
            && self.thumbnail_url == self.thumbnail_url
    }
}
impl Eq for FavoriteManga {}
