use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::auth::Claims;
use chrono::Local;
use sqlx::postgres::PgPool;
use std::str::FromStr;
use warp::Rejection;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Update {
    source: String,
    title: String,
    thumbnail_url: String,
    number: String,
    uploaded: chrono::NaiveDateTime,
}

impl Default for Update {
    fn default() -> Self {
        Update {
            source: "".to_string(),
            title: "".to_string(),
            thumbnail_url: "".to_string(),
            number: "".to_string(),
            uploaded: Local::now().naive_local(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdatesResponse {
    updates: Vec<Update>,
    status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateParam {
    page: i32,
}

pub async fn get_updates(
    claim: Claims,
    param: UpdateParam,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let limit = 10;
    let offset = (param.page * limit) - limit;

    let updates = sqlx::query_as!(
        Update, 
            r#"SELECT source.name AS source, manga.title AS title, 
        manga.thumbnail_url AS thumbnail_url, 
        chapter.uploaded AS uploaded,
        chapter.number AS number  FROM chapter
    JOIN manga ON manga.id = chapter.manga_id
    JOIN source ON source.id = manga.source_id
    JOIN favorite ON favorite.manga_id = manga.id
    JOIN "user" ON "user".id = favorite.user_id
    WHERE "user".username = $1 ORDER BY uploaded DESC
    LIMIT $2 OFFSET $3"#,
    claim.sub, limit as i64, offset as i64
        )
        .fetch_all(&db).await;

    let res = UpdatesResponse {
        updates: updates.unwrap(),
        status: "success".to_string(),
    };

    Ok(warp::reply::json(&res))
}
