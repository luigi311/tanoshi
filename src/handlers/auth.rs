use crate::auth::{auth::Auth, Claims, User};
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

pub async fn register(user: User, db: PgPool) -> Result<impl warp::Reply, Infallible> {
    let res = Auth::register(user, db).await;
    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        warp::http::StatusCode::CREATED,
    ))
}

pub async fn login(user: User, token: String, db: PgPool) -> Result<impl warp::Reply, Infallible> {
    let res = Auth::login(token, user, db).await;
    Ok(warp::reply::json(&res))
}

pub fn validate(secret: String, token: String) -> Option<Claims> {
    Auth::validate(secret, token)
}
