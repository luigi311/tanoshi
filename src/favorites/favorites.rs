use serde_json::json;

use crate::favorites::{AddFavoritesResponse, FavoriteManga, GetFavoritesResponse};
use crate::model::{Document, Manga};
use crate::scraper::Manga as ScrapedManga;
use postgres::Client;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Favorites {}

impl Favorites {
    pub fn new() -> Self {
        Favorites {}
    }

    pub fn get_favorites(&self, username: String, db: Arc<Mutex<Client>>) -> GetFavoritesResponse {
        let mut conn = db.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT manga.path, manga.title, source.name, manga.thumbnail_url FROM favorite
            JOIN user ON user.id = favorite.user_id
            JOIN manga ON manga.id = favorite.manga_id
            JOIN source ON source.id = manga.source_id
            WHERE user.username = ?1",
            )
            .unwrap();
        let rows = conn.query(&stmt, &[&username]).unwrap();
        let favorites: Vec<Manga> = rows
            .iter()
            .map(|row| Manga {
                path: row.get(0),
                title: row.get(1),
                source: row.get(2),
                thumbnail_url: row.get(3),
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
        db: Arc<Mutex<Client>>,
    ) -> AddFavoritesResponse {
        let mut conn = db.lock().unwrap();
        match conn.execute(
            "INSERT OR IGNORE INTO favorite(user_id, manga_id) \
        VALUES(
            (SELECT id FROM user WHERE username = ?1),
            (SELECT manga.id FROM manga JOIN source ON source.id = manga.source_id 
            WHERE source.name = ?2 AND manga.title = ?3)
        )",
            &[&username, &request.source, &request.title],
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
        db: Arc<Mutex<Client>>,
    ) -> AddFavoritesResponse {
        let mut conn = db.lock().unwrap();
        match conn.execute("DELETE FROM favorite 
        WHERE user_id = (SELECT id FROM user WHERE username = ?1)
        AND manga_id = (SELECT manga.id FROM manga JOIN source ON source.id = manga.source_id WHERE source.name = ?2 AND manga.title = ?3)",
        &[&username, &source, &title]) {
            Ok(_) => AddFavoritesResponse {
                status: "success".to_string(),
            },
            Err(e) => AddFavoritesResponse {
                status: format!("failed, reason: {}", e.to_string()),
            }
        }
    }
}
