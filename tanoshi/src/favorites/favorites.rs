use rusqlite::{params, Connection, OpenFlags};

use tanoshi_lib::manga::{AddFavoritesResponse, FavoriteManga, GetFavoritesResponse};

#[derive(Clone)]
pub struct Favorites {
    db_path: String,
    flags: OpenFlags,
}

impl Favorites {
    pub fn new() -> Self {
        Favorites {
            db_path: "./tanoshi.db".to_string(),
            flags: OpenFlags::SQLITE_OPEN_NO_MUTEX,
        }
    }

    fn connect_db(&self) -> Connection {
        Connection::open("./tanoshi.db").unwrap()
    }

    pub async fn get_favorites(&self, username: String) -> GetFavoritesResponse {
        let db = self.connect_db();
        let mut stmt = db
            .prepare(
                r#"SELECT
            manga.path AS path,
            manga.title AS title,
            favorite.manga_id AS manga_id,
            manga.thumbnail_url AS thumbnail_url
            FROM favorite
            JOIN "user" ON "user".id = favorite.user_id
            JOIN manga ON manga.id = favorite.manga_id
            WHERE "user".username = ?1"#,
            )
            .unwrap();
        let favorites = stmt
            .query_map(params![username], |row| {
                Ok(FavoriteManga {
                    path: row.get(0)?,
                    title: row.get(1)?,
                    manga_id: row.get(2)?,
                    thumbnail_url: row.get(3)?,
                })
            })
            .unwrap()
            .filter_map(|f| f.ok())
            .collect();

        GetFavoritesResponse {
            favorites: Some(favorites),
            status: "success".to_string(),
        }
    }

    pub async fn add_favorite(&self, username: String, manga_id: i32) -> AddFavoritesResponse {
        let db = self.connect_db();
        match db.execute(
            r#"INSERT INTO favorite(user_id, manga_id)
        VALUES(
            (SELECT id FROM "user" WHERE username = ?1),
            ?2
        ) ON CONFLICT DO NOTHING"#,
            params![username, manga_id],
        ) {
            Ok(_) => AddFavoritesResponse {
                status: "success".to_string(),
            },
            Err(e) => AddFavoritesResponse {
                status: format!("failed, reason: {}", e.to_string()),
            },
        }
    }

    pub async fn remove_favorites(&self, username: String, manga_id: i32) -> AddFavoritesResponse {
        let db = self.connect_db();
        match db.execute(
            r#"DELETE FROM favorite 
        WHERE user_id = (SELECT id FROM "user" WHERE username = ?1)
        AND manga_id = ?2"#,
            params![username, manga_id],
        ) {
            Ok(_) => AddFavoritesResponse {
                status: "success".to_string(),
            },
            Err(e) => AddFavoritesResponse {
                status: format!("failed, reason: {}", e.to_string()),
            },
        }
    }
}
