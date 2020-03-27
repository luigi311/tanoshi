use crate::auth::Claims;
use crate::filters::favorites::favorites;
use crate::model::Chapter;
use chrono::Local;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct History {
    source: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail_url: Option<String>,
    chapter: String,
    read: i32,
    at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryResponse {
    pub history: Vec<History>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryParam {
    page: i32,
}

pub async fn get_history(
    claim: Claims,
    param: HistoryParam,
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Infallible> {
    let limit = 10;
    let offset = (param.page * limit) - limit;

    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT source.name, manga.title, 
                   manga.thumbnail_url, chapter.number, 
                   history.last_page, history.at 
                   FROM chapter
                   JOIN manga ON manga.id = chapter.manga_id
                   JOIN source ON source.id = manga.source_id
                   JOIN history ON history.chapter_id = chapter.id
                   JOIN user ON user.id = history.user_id
                   WHERE user.username = ?1 ORDER BY at DESC
                   LIMIT ?2 OFFSET ?3 ",
        )
        .unwrap();
    let history_iter = stmt
        .query_map(params![claim.sub, limit, offset], |row| {
            Ok(History {
                source: row.get(0)?,
                title: row.get(1)?,
                thumbnail_url: row.get(2)?,
                chapter: row.get(3)?,
                read: row.get(4)?,
                at: row.get(5)?,
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
    request: History,
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
