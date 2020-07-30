use rusqlite::{params, Connection};

use tanoshi_lib::manga::{Manga, Params, SortByParam, SortOrderParam};
use tanoshi_lib::rest::{AddFavoritesResponse, GetMangasResponse};

#[derive(Clone)]
pub struct Favorites {
    database_path: String,
}

impl Favorites {
    pub fn new(database_path: String) -> Self {
        Favorites { database_path }
    }

    fn connect_db(&self) -> Connection {
        Connection::open(self.database_path.clone()).unwrap()
    }

    pub async fn get_favorites(&self, params: Params, username: String) -> GetMangasResponse {
        let db = self.connect_db();

        let sort_by = match params.sort_by.unwrap_or_default() {
            SortByParam::LastUpdated=> "last_updated",
            _=> "title"
        };

        let sort_order = match params.sort_order.unwrap_or_default() {
            SortOrderParam::Asc => "ASC",
            SortOrderParam::Desc => "DESC"
        };

        let mut stmt = db
            .prepare(format!(r#"SELECT
        manga.id AS id,
        manga.source AS source,
        manga.title AS title,
        CASE
            WHEN author IS NOT NULL THEN author
            ELSE ''
            END author,
        CASE
            WHEN status IS NOT NULL THEN status
            ELSE ''
            END status,
        CASE
            WHEN description IS NOT NULL THEN description
            ELSE ''
            END description,
        manga.path,
        thumbnail_url,
        CASE WHEN f.manga_id IS NULL
             THEN false
             ELSE
                 true
         END is_favorite
         FROM manga
         INNER JOIN favorite f on manga.id = f.manga_id AND f.user_id = (SELECT id FROM "user" WHERE username = ?1)
         JOIN (SELECT manga_id, MAX(uploaded) as last_updated FROM chapter GROUP BY manga_id) c on manga.id = c.manga_id
         ORDER BY {} {}"#, sort_by, sort_order).as_str()).unwrap();
        let mangas = stmt
            .query_map(params![username], |row| {
                let author = row
                    .get::<_, String>(3)?
                    .split(",")
                    .map(|a| a.to_string())
                    .collect();
                Ok(Manga {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    title: row.get(2)?,
                    author,
                    status: row.get(4)?,
                    description: row.get(5)?,
                    path: row.get(6)?,
                    thumbnail_url: row.get(7)?,
                    last_read: None,
                    last_page: None,
                    is_favorite: row.get(8)?,
                    genre: vec![],
                })
            })
            .unwrap()
            .filter_map(|f| f.ok())
            .collect();

        GetMangasResponse {
            mangas,
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
