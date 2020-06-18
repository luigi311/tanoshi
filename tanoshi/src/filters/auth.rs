use super::{with_admin_role, with_authorization};
use crate::auth::User;
use crate::handlers::auth as auth_handler;
use warp::Filter;

pub fn authentication(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    login(secret.clone())
        .or(register(secret.clone()))
        .or(user_list(secret.clone()))
        .or(modify_user_role(secret.clone()))
        .or(change_password(secret.clone()))
        .or(validate(secret))
}

pub fn login(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "login")
        .and(warp::post())
        .and(json_body())
        .and(with_secret(secret))
        .and_then(auth_handler::login)
}

pub fn register(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "register")
        .and(warp::post())
        .and(json_body())
        .and(with_admin_role(secret))
        .and_then(auth_handler::register)
}

pub fn user_list(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "user")
        .and(warp::get())
        .and(with_admin_role(secret))
        .and_then(auth_handler::user_list)
}

pub fn modify_user_role(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "user" / "role")
        .and(warp::put())
        .and(json_body())
        .and(with_admin_role(secret))
        .and_then(auth_handler::modify_user_role)
}

pub fn change_password(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "user" / "password")
        .and(warp::put())
        .and(text_body())
        .and(with_authorization(secret))
        .and_then(auth_handler::change_password)
}

pub fn validate(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "validate")
        .and(warp::get())
        .and(with_authorization(secret))
        .map(|claim| Ok(warp::reply::json(&claim)))
}

fn with_secret(
    secret: String,
) -> impl Filter<Extract = (String,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || secret.clone())
}

fn json_body() -> impl Filter<Extract = (User,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

fn text_body() -> impl Filter<Extract = (String,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16)
        .and(warp::body::bytes())
        .map(|bytes: bytes::Bytes| {
            String::from_utf8(bytes.to_vec()).expect("failed to parse password")
        })
}
