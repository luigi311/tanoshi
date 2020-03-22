use crate::auth::auth::Auth;
use crate::auth::Claims;
use crate::filters::settings::settings::auth_handler;
use sled::Tree;
use warp::Filter;

pub mod auth;
pub mod favorites;
pub mod history;
pub mod manga;
pub mod settings;

pub fn with_db(
    db: Tree,
) -> impl Filter<Extract = (Tree,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

pub fn with_authorization(
    auth: Auth,
) -> impl Filter<Extract = (Claims,), Error = warp::reject::Rejection> + Clone {
    warp::header::header("authorization")
        .map(move |token: String| auth_handler::validate(token, auth.clone()).unwrap())
}
