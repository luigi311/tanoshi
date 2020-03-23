use crate::auth::auth::Auth;
use crate::auth::Claims;
use crate::handlers::auth as auth_handler;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use warp::Filter;

pub mod auth;
pub mod favorites;
pub mod history;
pub mod manga;
pub mod updates;

pub fn with_db(
    db: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = (Arc<Mutex<Connection>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

pub fn with_authorization(
    secret: String,
) -> impl Filter<Extract = (Claims,), Error = warp::reject::Rejection> + Clone {
    warp::header::header("authorization")
        .map(move |token: String| auth_handler::validate(secret.clone(), token.clone()).unwrap())
}
