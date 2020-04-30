use serde_json::json;

use crate::favorites::{AddFavoritesResponse, FavoriteManga, GetFavoritesResponse};
use crate::model::{Document, Manga};
use tanoshi::manga::Manga as ScrapedManga;
use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct Favorites {}

impl Favorites {
    pub fn new() -> Self {
        Favorites {}
    }

    pub async fn get_favorites(&self, username: String, db: PgPool) -> GetFavoritesResponse {
        let favorites = sqlx::query_as!(
            Manga,
            r#"SELECT manga.path AS path, manga.title AS title, source.name AS source, manga.thumbnail_url AS thumbnail_url FROM favorite
            JOIN "user" ON "user".id = favorite.user_id
            JOIN manga ON manga.id = favorite.manga_id
            JOIN source ON source.id = manga.source_id
            WHERE "user".username = $1"#,
            username
            ).fetch_all(&db).await;

        GetFavoritesResponse {
            favorites: favorites.unwrap(),
            status: "success".to_string(),
        }
    }

    pub async fn add_favorite(
        &self,
        username: String,
        request: FavoriteManga,
        db: PgPool,
    ) -> AddFavoritesResponse {
        match sqlx::query!(
            r#"INSERT INTO favorite(user_id, manga_id)
        VALUES(
            (SELECT id FROM "user" WHERE username = $1),
            (SELECT manga.id FROM manga JOIN source ON source.id = manga.source_id 
            WHERE source.name = $2 AND manga.title = $3)
        ) ON CONFLICT DO NOTHING"#,
            username, request.source, request.title,
        ).execute(&db).await {
            Ok(_) => AddFavoritesResponse {
                status: "success".to_string(),
            },
            Err(e) => AddFavoritesResponse {
                status: format!("failed, reason: {}", e.to_string()),
            },
        }
    }

    pub async fn remove_favorites(
        &self,
        username: String,
        source: String,
        title: String,
        db: PgPool,
    ) -> AddFavoritesResponse {
        match sqlx::query!(r#"DELETE FROM favorite 
        WHERE user_id = (SELECT id FROM "user" WHERE username = $1)
        AND manga_id = (SELECT manga.id FROM manga 
        JOIN source ON source.id = manga.source_id WHERE source.name = $2 AND manga.title = $3)"#,
        username, source, title).execute(&db).await {
            Ok(_) => AddFavoritesResponse {
                status: "success".to_string(),
            },
            Err(e) => AddFavoritesResponse {
                status: format!("failed, reason: {}", e.to_string()),
            }
        }
    }
}
