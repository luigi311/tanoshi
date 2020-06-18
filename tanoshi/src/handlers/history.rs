use crate::auth::Claims;
use crate::history::{History, HistoryParam};
use rusqlite::{params, Connection};
use std::convert::Infallible;
use tanoshi_lib::manga::HistoryRequest;

fn connect_db() -> Connection {
    Connection::open("./tanoshi.db").unwrap()
}

pub async fn get_history(
    claim: Claims,
    param: HistoryParam,
    history: History,
) -> Result<impl warp::Reply, Infallible> {
    history.get_history(claim, param).await
}

pub async fn add_history(
    claim: Claims,
    request: HistoryRequest,
    history: History,
) -> Result<impl warp::Reply, Infallible> {
    history.add_history(claim, request).await
}
