use crate::auth::Claims;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgRow};
use std::convert::Infallible;
use tanoshi::manga::{HistoryRequest, HistoryResponse, History};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryParam {
    page: i32,
}

pub async fn get_history(
    claim: Claims,
    param: HistoryParam,
    db: PgPool,
) -> Result<impl warp::Reply, Infallible> {
    let limit: i64 = 10;
    let offset: i64 = (param.page as i64 * limit) - limit;
    let histories = sqlx::query(
            r#"SELECT 
            manga.title AS title, 
            manga.thumbnail_url as thumbnail_url, 
            chapter.number AS chapter, 
            chapter.id AS chapter_id,
            history.last_page AS read, 
            history.at AS at,
            manga.id
            FROM chapter
            JOIN manga ON manga.id = chapter.manga_id
            JOIN history ON history.chapter_id = chapter.id
            JOIN "user" ON "user".id = history.user_id
            WHERE "user".username = $1 ORDER BY at DESC
            LIMIT $2 OFFSET $3"#
        )
        .bind(claim.sub)
        .bind(limit)
        .bind(offset)
        .map(|row: PgRow| 
            History {
                title: row.get(0),
                thumbnail_url: row.get(1),
                chapter: row.get(2),
                chapter_id: row.get(3),
                read: row.get(4),
                at: row.get(5),
                manga_id: row.get(6),
                days: None,
                show_sep: None,
            })
        .fetch_all(&db).await;
    
    let res = HistoryResponse {
        history: histories.unwrap(),
        status: "success".to_string(),
    };

    if res.status != "success" {
        return Ok(warp::reply::with_status(
            warp::reply::json(&res),
            warp::http::status::StatusCode::BAD_REQUEST,
        ));
    }
    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        warp::http::status::StatusCode::OK,
    ))
}

pub async fn add_history(
    claim: Claims,
    request: HistoryRequest,
    db: PgPool,
) -> Result<impl warp::Reply, Infallible> {
    let reply = match sqlx::query!(
        r#"INSERT INTO history(user_id, chapter_id, last_page, at)
        VALUES(
        (SELECT id FROM "user" WHERE username = $1),
        $2,
        $3,
        $4)
        ON CONFLICT(user_id, chapter_id)
        DO UPDATE SET last_page = excluded.last_page,
        at = excluded.at,
        updated = CURRENT_TIMESTAMP"#,
           claim.sub,
           request.chapter_id,
           request.read,
           request.at,
    ).execute(&db).await 
    {
        Ok(_) => warp::reply::with_status(
            warp::reply::json(&HistoryResponse {
                history: vec![],
                status: "success".to_string(),
            }),
            warp::http::status::StatusCode::OK,
        ),
        Err(e) => warp::reply::with_status(
            warp::reply::json(&HistoryResponse {
                history: vec![],
                status: format!("failed, reason: {}", e.to_string()),
            }),
            warp::http::status::StatusCode::BAD_REQUEST,
        ),
    };

    Ok(reply)
}
