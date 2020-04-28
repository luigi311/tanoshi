use crate::auth::auth::Auth;
use crate::auth::User;
use crate::filters::with_db;
use crate::handlers::auth as auth_handler;
use sqlx::postgres::PgPool;
use std::sync::{Arc, Mutex};
use warp::Filter;
use super::with_authorization;

pub fn authentication(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    login(secret.clone(), db.clone()).or(register(secret.clone(), db.clone())).or(validate(secret))
}

pub fn login(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "login")
        .and(warp::post())
        .and(json_body())
        .and(with_secret(secret))
        .and(with_db(db))
        .and_then(auth_handler::login)
}

pub fn register(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "register")
        .and(warp::post())
        .and(json_body())
        .and(with_db(db))
        .and_then(auth_handler::register)
}

pub fn validate(
    secret: String
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "validate")
    .and(warp::get())
    .and(with_authorization(secret))
    .map(|_| {
        Ok(warp::reply())
    })
}

fn with_secret(
    secret: String,
) -> impl Filter<Extract = (String,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || secret.clone())
}

fn json_body() -> impl Filter<Extract = (User,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}
