use crate::auth::Claims;
use crate::filters::favorites::favorites;
use crate::history::{history::History, HistoryRequest};
use rusqlite::Connection;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

pub async fn get_history(
    source: String,
    title: String,
    claim: Claims,
    history: History,
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Infallible> {
    let res = history.get_history(claim.sub, source, title, db);
    let mut status = warp::http::status::StatusCode::OK;
    if res.status != "success" {
        status = warp::http::status::StatusCode::BAD_REQUEST;
    }
    Ok(warp::reply::with_status(warp::reply::json(&res), status))
}

pub async fn add_history(
    claim: Claims,
    request: HistoryRequest,
    history: History,
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Infallible> {
    let res = history.add_history(claim.sub, request, db);
    Ok(warp::reply::json(&res))
}
