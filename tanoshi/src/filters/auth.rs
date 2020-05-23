use crate::auth::User;
use crate::filters::with_db;
use crate::handlers::auth as auth_handler;
use sqlx::postgres::PgPool;
use warp::Filter;
use super::{with_authorization, with_admin_role};

pub fn authentication(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    login(secret.clone(), db.clone()).or(register(secret.clone(), db.clone())).or(user_list(secret.clone(), db.clone())).or(validate(secret))
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
        .and(with_admin_role(secret))
        .and(with_db(db))
        .and_then(auth_handler::register)
}

pub fn user_list(secret: String, db: PgPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "user")
        .and(warp::get())
        .and(with_admin_role(secret))
        .and(with_db(db))
        .and_then(auth_handler::user_list)
}

pub fn validate(
    secret: String
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "validate")
    .and(warp::get())
    .and(with_authorization(secret))
    .map(|claim| {
        Ok(warp::reply::json(&claim))
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
