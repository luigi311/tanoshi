use crate::auth::{auth::Auth, Claims, User};
use serde_json::json;

use std::convert::Infallible;

pub async fn register(user: User, _claims: Claims) -> Result<impl warp::Reply, Infallible> {
    let res = Auth::register(user).await;
    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        warp::http::StatusCode::CREATED,
    ))
}

pub async fn login(user: User, token: String) -> Result<impl warp::Reply, Infallible> {
    let res = Auth::login(token, user).await;
    if res.status == "failed" {
        return Ok(warp::reply::with_status(
            warp::reply::json(&res),
            warp::http::StatusCode::UNAUTHORIZED,
        ));
    }
    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        warp::http::StatusCode::OK,
    ))
}

pub async fn user_list(_claims: Claims) -> Result<impl warp::Reply, Infallible> {
    let res = Auth::user_list().await;
    Ok(warp::reply::json(&json!({ "users": res })))
}

pub async fn modify_user_role(user: User, _claims: Claims) -> Result<impl warp::Reply, Infallible> {
    match Auth::modify_user_role(user).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply(),
            warp::http::status::StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply(),
            warp::http::status::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn change_password(
    password: String,
    claims: Claims,
) -> Result<impl warp::Reply, Infallible> {
    match Auth::change_password(claims.sub, password).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply(),
            warp::http::status::StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply(),
            warp::http::status::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub fn validate(secret: String, token: String) -> Option<Claims> {
    Auth::validate(secret, token)
}
