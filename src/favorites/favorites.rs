use serde_json::json;

use crate::favorites::{AddFavoritesResponse, FavoriteManga, GetFavoritesResponse};
use crate::model::{Document, Manga};
use crate::scraper::Manga as ScrapedManga;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Favorites {}

impl Favorites {
    pub fn new() -> Self {
        Favorites {}
    }

    pub fn get_favorites(
        &self,
        username: String,
        db: Arc<Mutex<Connection>>,
    ) -> GetFavoritesResponse {
        let conn = db.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT manga.path, manga.title, source.name, manga.thumbnail_url FROM favorite
            JOIN user ON user.id = favorite.user_id
            JOIN manga ON manga.id = favorite.manga_id
            JOIN source ON source.id = manga.source_id
            WHERE user.username = ?1",
            )
            .unwrap();
        let fav_iter = stmt
            .query_map(params![username], |row| {
                Ok(Manga {
                    path: row.get(0)?,
                    title: row.get(1)?,
                    source: row.get(2)?,
                    thumbnail_url: row.get(3)?,
                })
            })
            .unwrap();

        let mut favorites = vec![];
        for manga in fav_iter {
            favorites.push(manga.unwrap());
        }

        GetFavoritesResponse {
            favorites,
            status: "success".to_string(),
        }
    }

    pub fn add_favorite(
        &self,
        username: String,
        request: FavoriteManga,
        db: Arc<Mutex<Connection>>,
    ) -> AddFavoritesResponse {
        let conn = db.lock().unwrap();
        match conn.execute(
            "INSERT OR IGNORE INTO favorite(user_id, manga_id) \
        VALUES(
            (SELECT id FROM user WHERE username = ?1),
            (SELECT manga.id FROM manga JOIN source ON source.id = manga.source_id 
            WHERE source.name = ?2 AND manga.title = ?3)
        )",
            params![username, request.source, request.title],
        ) {
            Ok(_) => AddFavoritesResponse {
                status: "success".to_string(),
            },
            Err(e) => AddFavoritesResponse {
                status: format!("failed, reason: {}", e.to_string()),
            },
        }
    }

    pub fn remove_favorites(
        &self,
        username: String,
        source: String,
        title: String,
        db: Arc<Mutex<Connection>>,
    ) -> AddFavoritesResponse {
        let conn = db.lock().unwrap();
        match conn.execute("DELETE FROM favorite 
        WHERE user_id = (SELECT id FROM user WHERE username = ?1)
        AND manga_id = (SELECT manga.id FROM manga JOIN source ON source.id = manga.source_id WHERE source.name = ?2 AND manga.title = ?3)",
        params![username, source, title]) {
            Ok(_) => AddFavoritesResponse {
                status: "success".to_string(),
            },
            Err(e) => AddFavoritesResponse {
                status: format!("failed, reason: {}", e.to_string()),
            }
        }
    }
}
