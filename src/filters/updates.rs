use crate::filters::{with_authorization, with_db};
use crate::handlers::updates;
use crate::handlers::updates::UpdateParam;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use warp::{path, Filter};

pub fn updates(
    secret: String,
    db: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    get_updates(secret.clone(), db.clone())
}

pub fn get_updates(
    secret: String,
    db: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "updates")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<UpdateParam>())
        .and(with_db(db))
        .and_then(updates::get_updates)
}
