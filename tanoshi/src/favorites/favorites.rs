use sqlx::postgres::PgPool;
use tanoshi::manga::{AddFavoritesResponse, FavoriteManga, GetFavoritesResponse};

#[derive(Clone)]
pub struct Favorites {}

impl Favorites {
    pub fn new() -> Self {
        Favorites {}
    }

    pub async fn get_favorites(&self, username: String, db: PgPool) -> GetFavoritesResponse {
        match sqlx::query_as!(
            FavoriteManga,
            r#"SELECT 
            manga.path AS path, 
            manga.title AS title, 
            favorite.manga_id AS manga_id, 
            manga.thumbnail_url AS thumbnail_url 
            FROM favorite
            JOIN "user" ON "user".id = favorite.user_id
            JOIN manga ON manga.id = favorite.manga_id
            WHERE "user".username = $1"#,
            username
        )
        .fetch_all(&db)
        .await
        {
            Ok(favs) => GetFavoritesResponse {
                favorites: Some(favs),
                status: "success".to_string(),
            },
            Err(e) => GetFavoritesResponse {
                favorites: None,
                status: e.to_string(),
            },
        }
    }

    pub async fn add_favorite(
        &self,
        username: String,
        manga_id: i32,
        db: PgPool,
    ) -> AddFavoritesResponse {
        match sqlx::query!(
            r#"INSERT INTO favorite(user_id, manga_id)
        VALUES(
            (SELECT id FROM "user" WHERE username = $1),
            $2
        ) ON CONFLICT DO NOTHING"#,
            username,
            manga_id,
        )
        .execute(&db)
        .await
        {
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
        manga_id: i32,
        db: PgPool,
    ) -> AddFavoritesResponse {
        match sqlx::query!(
            r#"DELETE FROM favorite 
        WHERE user_id = (SELECT id FROM "user" WHERE username = $1)
        AND manga_id = $2"#,
            username,
            manga_id
        )
        .execute(&db)
        .await
        {
            Ok(_) => AddFavoritesResponse {
                status: "success".to_string(),
            },
            Err(e) => AddFavoritesResponse {
                status: format!("failed, reason: {}", e.to_string()),
            },
        }
    }
}
