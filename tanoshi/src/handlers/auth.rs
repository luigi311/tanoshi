use crate::auth::{auth::Auth, Claims, User};
use serde_json::json;

use std::convert::Infallible;

pub async fn register(
    user: User,
    _claims: Claims,
    auth: Auth,
) -> Result<impl warp::Reply, Infallible> {
    let res = auth.register(user).await;
    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        warp::http::StatusCode::CREATED,
    ))
}

pub async fn login(user: User, token: String, auth: Auth) -> Result<impl warp::Reply, Infallible> {
    let res = auth.login(token, user).await;
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

pub async fn user_list(_claims: Claims, auth: Auth) -> Result<impl warp::Reply, Infallible> {
    let res = auth.user_list().await;
    Ok(warp::reply::json(&json!({ "users": res })))
}

pub async fn modify_user_role(
    user: User,
    _claims: Claims,
    auth: Auth,
) -> Result<impl warp::Reply, Infallible> {
    match auth.modify_user_role(user).await {
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
    auth: Auth,
) -> Result<impl warp::Reply, Infallible> {
    match auth.change_password(claims.sub, password).await {
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
