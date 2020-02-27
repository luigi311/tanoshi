use crate::favorites::FavoritesResponse;
use crate::scraper::{Chapter, Manga};
use sled::Db;

#[derive(Clone)]
pub struct Favorites {
    db: Db,
}

impl Favorites {
    pub fn new() -> Self {
        let db = sled::open("./db/favorites").unwrap();
        Favorites { db }
    }

    pub fn get_favorites(&self, username: String) -> FavoritesResponse {
        let mangas: Vec<Manga> = match self
            .db
            .get(format!("favorites:{}:mangas", username))
            .unwrap()
        {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap(),
            None => vec![],
        };
        FavoritesResponse {
            favorites: Some(mangas),
            status: "success".to_string(),
        }
    }

    pub fn add_favorite(&self, username: String, manga: Manga) -> FavoritesResponse {
        self.db.fetch_and_update(
            format!("favorites:{}:mangas", username),
            |fav: Option<&[u8]>| {
                let mut mangas: Vec<Manga> = match fav {
                    Some(bytes) => {
                        let manga_fav: Vec<Manga> = serde_json::from_slice(bytes).unwrap();
                        manga_fav
                    }
                    None => vec![],
                };
                mangas.push(manga.clone());
                serde_json::to_vec(&mangas).ok()
            },
        );
        FavoritesResponse {
            favorites: None,
            status: "success".to_string(),
        }
    }
}
