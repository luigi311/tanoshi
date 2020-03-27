use crate::auth::Claims;
use crate::filters::favorites::favorites;
use crate::model::Chapter;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryRequest {
    source: String,
    title: String,
    chapter: String,
    read: i32,
    at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryResponse {
    pub history: Vec<Chapter>,
    pub status: String,
}

pub async fn get_history(
    source: String,
    title: String,
    claim: Claims,
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Infallible> {
    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT chapter.number, history.last_page FROM history
            JOIN user ON user.id = history.user_id
            JOIN chapter ON chapter.id = history.chapter_id
            JOIN manga ON manga.id = chapter.manga_id
            JOIN source ON source.id = manga.source_id
            WHERE user.username = ?1 AND source.name = ?2 AND manga.title = ?3",
        )
        .unwrap();
    let history_iter = stmt
        .query_map(params![claim.sub, source, title], |row| {
            Ok(Chapter {
                path: row.get(0)?,
                read: row.get(1)?,
            })
        })
        .unwrap();

    let mut histories = vec![];
    for ch in history_iter {
        histories.push(ch.unwrap())
    }

    let res = HistoryResponse {
        history: histories,
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
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Infallible> {
    let conn = db.lock().unwrap();
    let reply = match conn.execute(
        "INSERT INTO history(user_id, chapter_id, last_page, at) \
        VALUES(\
        (SELECT id FROM user WHERE username = ?1), \
        (SELECT chapter.id FROM chapter \
        JOIN manga ON manga.id = chapter.manga_id \
        JOIN source ON source.id = manga.source_id \
        WHERE source.name = ?2 \
        AND manga.title = ?3 \
        AND chapter.number = ?4), \
        ?5, \
        ?6) \
        ON CONFLICT(user_id, chapter_id) \
         DO UPDATE SET last_page = excluded.last_page, \
         at = excluded.at, \
         updated = CURRENT_TIMESTAMP",
        params![
            claim.sub,
            request.source,
            request.title,
            request.chapter,
            request.read,
            request.at
        ],
    ) {
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
