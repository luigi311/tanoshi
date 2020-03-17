use crate::favorites::{AddFavoritesResponse, FavoriteManga, GetFavoritesResponse};
use crate::model::Manga;
use crate::scraper::Manga as ScrapedManga;
use serde_json::json;
use sled::{Db, IVec};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Favorites {}

impl Favorites {
    pub fn new() -> Self {
        Favorites {}
    }

    pub fn get_favorites(&self, username: String, db: Db) -> GetFavoritesResponse {
        let mangas: Vec<Manga> = match db.get(format!("manga#{}", username)).unwrap() {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap(),
            None => vec![],
        };

        let mut favs = vec![];
        for m in mangas {
            let key = format!("{}#{}", m.source.clone(), m.path.clone());
            let res = db.get(key).unwrap();
            let res: ScrapedManga = match res {
                Some(bytes) => serde_json::from_slice(&bytes).unwrap(),
                None => ScrapedManga::default(),
            };

            favs.push(FavoriteManga {
                source: m.source,
                title: m.title,
                path: m.path,
                thumbnail_url: res.thumbnail_url,
            })
        }

        GetFavoritesResponse {
            favorites: favs,
            status: "success".to_string(),
        }
    }

    pub fn add_favorite(
        &self,
        username: String,
        manga: FavoriteManga,
        db: Db,
    ) -> AddFavoritesResponse {
        let manga = Manga {
            path: manga.path,
            title: manga.title,
            source: manga.source,
        };

        let status =
            match db.fetch_and_update(format!("manga#{}", username), |result: Option<&[u8]>| {
                let mut value: Vec<Manga> = match result {
                    Some(val) => serde_json::from_slice(val).unwrap(),
                    None => vec![],
                };

                if !value.contains(&manga.clone()) {
                    value.push(manga.clone());
                }
                serde_json::to_vec(&value).ok()
            }) {
                Ok(_) => "success".to_string(),
                Err(e) => format!("failed add favorite, msg: {}", e.to_string()),
            };
        AddFavoritesResponse { status }
    }

    pub fn remove_favorites(
        &self,
        username: String,
        manga: FavoriteManga,
        db: Db,
    ) -> AddFavoritesResponse {
        let manga = Manga {
            path: manga.path,
            title: manga.title,
            source: manga.source,
        };
        let status =
            match db.fetch_and_update(format!("manga#{}", username), move |fav: Option<&[u8]>| {
                let mut mangas: Vec<Manga> = match fav {
                    Some(bytes) => {
                        let manga_fav: Vec<Manga> = serde_json::from_slice(bytes).unwrap();
                        manga_fav
                    }
                    None => vec![],
                };

                match mangas.iter().position(|m| m.clone() == manga.clone()) {
                    Some(index) => Some(mangas.remove(index)),
                    None => None,
                };
                serde_json::to_vec(&mangas).ok()
            }) {
                Ok(_) => "success".to_string(),
                Err(_) => "failed".to_string(),
            };
        AddFavoritesResponse { status }
    }
}
