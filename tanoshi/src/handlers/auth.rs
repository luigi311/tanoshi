use crate::auth::{auth::Auth, Claims, User};
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use serde_json::json;

pub async fn register(user: User, _claims: Claims, db: PgPool) -> Result<impl warp::Reply, Infallible> {
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

pub async fn user_list(_claims: Claims, db: PgPool) -> Result<impl warp::Reply, Infallible> {
   let res = Auth::user_list(db).await;
   Ok(warp::reply::json(&json!({"users" : res})))
}

pub async fn modify_user_role(user: User, _claims: Claims, db: PgPool) -> Result<impl warp::Reply, Infallible> {
   Auth::modify_user_role(user, db).await;
   Ok(warp::reply())
}

pub fn validate(secret: String, token: String) -> Option<Claims> {
    Auth::validate(secret, token)
}
