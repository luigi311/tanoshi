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
    Ok(warp::reply::json(&res))
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
