use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::auth::Claims;
use chrono::Local;
use rusqlite::{params, Connection};
use std::str::FromStr;
use warp::Rejection;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Update {
    source: String,
    title: String,
    thumbnail_url: String,
    number: String,
    uploaded: chrono::DateTime<Local>,
}

impl Default for Update {
    fn default() -> Self {
        Update {
            source: "".to_string(),
            title: "".to_string(),
            thumbnail_url: "".to_string(),
            number: "".to_string(),
            uploaded: Local::now(),
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
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Rejection> {
    let limit = 10;
    let offset = (param.page * limit) - limit;

    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT source.name, manga.title, 
        manga.thumbnail_url, chapter.number, 
        chapter.uploaded FROM chapter
    JOIN manga ON manga.id = chapter.manga_id
    JOIN source ON source.id = manga.source_id
    JOIN favorite ON favorite.manga_id = manga.id
    JOIN user ON user.id = favorite.user_id
    WHERE user.username = ?1 ORDER BY uploaded DESC
    LIMIT ?2 OFFSET ?3 ",
        )
        .unwrap();

    let updates_iter = stmt
        .query_map(params![claim.sub, limit, offset], |row| {
            Ok(Update {
                source: row.get(0)?,
                title: row.get(1)?,
                thumbnail_url: row.get(2)?,
                number: row.get(3)?,
                uploaded: row.get(4)?,
            })
        })
        .unwrap();

    let mut updates = vec![];
    for update in updates_iter {
        updates.push(update.unwrap());
    }

    let res = UpdatesResponse {
        updates: updates,
        status: "success".to_string(),
    };

    Ok(warp::reply::json(&res))
}
