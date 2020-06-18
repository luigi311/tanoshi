use serde_json::json;

use warp::Filter;

use crate::auth::Claims;
use crate::handlers::auth as auth_handler;

pub mod auth;
pub mod favorites;
pub mod history;
pub mod manga;
pub mod updates;

#[derive(Debug)]
pub struct ExpiredOrInvalidToken;

impl warp::reject::Reject for ExpiredOrInvalidToken {}

pub fn with_authorization(
    secret: String,
) -> impl Filter<Extract = (Claims,), Error = warp::reject::Rejection> + Clone {
    warp::header::header("authorization")
        .map(move |token: String| auth_handler::validate(secret.clone(), token.to_string()))
        .and_then(|claim: Option<Claims>| async move {
            match claim {
                Some(claim) => Ok(claim),
                None => Err(warp::reject::custom(ExpiredOrInvalidToken)),
            }
        })
}

pub fn with_admin_role(
    secret: String,
) -> impl Filter<Extract = (Claims,), Error = warp::reject::Rejection> + Clone {
    warp::header::header("authorization")
        .map(move |token: String| auth_handler::validate(secret.clone(), token.to_string()))
        .and_then(|claim: Option<Claims>| async move {
            match claim {
                Some(claim) => match claim.role.as_str() {
                    "ADMIN" => Ok(claim),
                    _ => Err(warp::reject::custom(ExpiredOrInvalidToken)),
                },
                None => Err(warp::reject::custom(ExpiredOrInvalidToken)),
            }
        })
}

pub async fn handle_rejection(
    err: warp::reject::Rejection,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let code;
    let message;

    if err.is_not_found() {
        return Err(warp::reject());
    } else if let Some(ExpiredOrInvalidToken) = err.find() {
        code = warp::http::StatusCode::UNAUTHORIZED;
        message = "Unauthorized".to_string();
    } else if let Some(e) = err.find::<warp::reject::MissingHeader>() {
        code = warp::http::StatusCode::UNAUTHORIZED;
        message = format!("Mission {} header", e.clone());
    } else {
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Unhandled".to_string();
    }

    error!("code: {}, message: {:?}", code.clone(), err);

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "message": message })),
        code,
    ))
}
