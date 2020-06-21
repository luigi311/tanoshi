use crate::auth::Claims;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tanoshi_lib::manga::{Update as UpdateModel, UpdatesResponse};
use warp::Rejection;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateParam {
    page: i32,
}

#[derive(Debug, Clone)]
pub struct Update {
    database_path: String,
}

impl Update {
    pub fn new(database_path: String) -> Self {
        Self { database_path }
    }

    fn connect_db(&self) -> Connection {
        Connection::open(self.database_path.clone()).unwrap()
    }

    pub async fn get_updates(
        &self,
        claim: Claims,
        param: UpdateParam,
    ) -> Result<impl warp::Reply, Rejection> {
        let db = self.connect_db();
        let limit = 10;
        let offset = (param.page * limit) - limit;
        let mut stmt = db
            .prepare(
                r#"SELECT 
                manga.title AS title, 
                manga.thumbnail_url AS thumbnail_url, 
                chapter.uploaded AS uploaded,
                chapter.number AS number,
                chapter.id AS chapter_id,
                manga.id
                FROM chapter
                JOIN manga ON manga.id = chapter.manga_id
                JOIN source ON source.id = manga.source_id
                JOIN favorite ON favorite.manga_id = manga.id
                JOIN "user" ON "user".id = favorite.user_id
                WHERE "user".username = $1 ORDER BY uploaded DESC
                LIMIT $2 OFFSET $3"#,
            )
            .unwrap();
        let updates = stmt
            .query_map(params![claim.sub, limit as i64, offset as i64], |row| {
                Ok(UpdateModel {
                    title: row.get(0)?,
                    thumbnail_url: row.get(1)?,
                    uploaded: row.get(2)?,
                    number: row.get(3)?,
                    chapter_id: row.get(4)?,
                    manga_id: row.get(5)?,
                    days: None,
                    show_sep: None,
                })
            })
            .unwrap()
            .filter_map(|u| u.ok())
            .collect();

        let res = UpdatesResponse {
            updates,
            status: "success".to_string(),
        };

        Ok(warp::reply::json(&res))
    }
}
