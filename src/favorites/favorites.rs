use std::collections::BTreeMap;

use serde_json::json;
use sled::{IVec, Tree};

use crate::favorites::{AddFavoritesResponse, FavoriteManga, GetFavoritesResponse};
use crate::model::{Document, Manga};
use crate::scraper::Manga as ScrapedManga;

#[derive(Clone)]
pub struct Favorites {}

impl Favorites {
    pub fn new() -> Self {
        Favorites {}
    }

    pub fn get_favorites(&self, username: String, db: Tree) -> GetFavoritesResponse {
        let favorites: Vec<Document> = db
            .scan_prefix(format!("{}:favorites", username))
            .map(|el| {
                let (_, v) = el.unwrap();
                serde_json::from_slice(&v).unwrap()
            })
            .collect();

        GetFavoritesResponse {
            favorites,
            status: "success".to_string(),
        }
    }

    pub fn add_favorite(
        &self,
        username: String,
        request: FavoriteManga,
        library_tree: Tree,
        scraper_tree: Tree,
    ) -> AddFavoritesResponse {
        let manga_path: String = match scraper_tree
            .get(format!("{}:{}", &request.source, &request.title))
            .expect("failed to get manga")
        {
            Some(ret) => String::from_utf8(ret.to_vec()).unwrap(),
            None => {
                return AddFavoritesResponse {
                    status: "Not found".to_string(),
                }
            }
        };

        let m = Manga {
            path: manga_path,
            title: String::from_utf8(base64::decode(&request.title).unwrap()).unwrap(),
            source: request.source.clone(),
        };

        library_tree.merge(
            format!(
                "{}:favorites:{}:{}",
                username, &request.source, &request.title
            ),
            serde_json::to_vec(&m).unwrap(),
        );

        AddFavoritesResponse {
            status: "success".to_string(),
        }
    }

    pub fn remove_favorites(
        &self,
        username: String,
        manga: FavoriteManga,
        db: Tree,
    ) -> AddFavoritesResponse {
        let manga = Manga {
            path: "".to_string(),
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
