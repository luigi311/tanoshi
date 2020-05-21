use serde::{Deserialize, Serialize};
use crate::auth::Claims;
use chrono::Local;
use sqlx::Row;
use sqlx::postgres::{PgPool, PgRow};
use warp::Rejection;
use tanoshi_lib::manga::{Update, UpdatesResponse};

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

    let updates = sqlx::query(
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
            LIMIT $2 OFFSET $3"#
        )
        .bind(claim.sub)
        .bind(limit as i64)
        .bind(offset as i64)
        .map(|row: PgRow| Update {
            title: row.get(0),
            thumbnail_url: row.get(1),
            uploaded: row.get(2),
            number: row.get(3),
            chapter_id: row.get(4),
            manga_id: row.get(5),
            days: None,
            show_sep: None,
        })
        .fetch_all(&db).await;

    let res = UpdatesResponse {
        updates: updates.unwrap(),
        status: "success".to_string(),
    };

    Ok(warp::reply::json(&res))
}
