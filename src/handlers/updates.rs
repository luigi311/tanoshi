use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::auth::Claims;
use chrono::Local;
use postgres::Client;
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
    db: Arc<Mutex<Client>>,
) -> Result<impl warp::Reply, Rejection> {
    let limit = 10;
    let offset = (param.page * limit) - limit;

    let mut conn = db.lock().unwrap();
    let stmt = conn
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

    let rows = conn.query(&stmt, &[&claim.sub, &limit, &offset]).unwrap();

    let mut updates = rows
        .iter()
        .map(|row| Update {
            source: row.get(0),
            title: row.get(1),
            thumbnail_url: row.get(2),
            number: row.get(3),
            uploaded: row.get(4),
        })
        .collect::<Vec<Update>>();

    let res = UpdatesResponse {
        updates: updates,
        status: "success".to_string(),
    };

    Ok(warp::reply::json(&res))
}
