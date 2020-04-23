use crate::auth::auth::Auth;
use crate::auth::Claims;
use crate::favorites::favorites::Favorites;
use crate::favorites::FavoriteManga;
use crate::filters::{with_authorization, with_db};
use crate::handlers::auth as auth_handler;
use crate::handlers::favorites as favorite_handler;
use sqlx::postgres::PgPool;
use std::sync::{Arc, Mutex};
use warp::Filter;

pub fn favorites(
    secret: String,
    fav: Favorites,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    get_favorites(secret.clone(), fav.clone(), db.clone())
        .or(add_favorites(secret.clone(), fav.clone(), db.clone()))
        .or(remove_favorites(secret, fav, db))
}

fn get_favorites(
    secret: String,
    fav: Favorites,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "favorites")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(with_favorites(fav))
        .and(with_db(db))
        .and_then(favorite_handler::get_favorites)
}

fn add_favorites(
    secret: String,
    fav: Favorites,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "favorites")
        .and(warp::post())
        .and(with_authorization(secret))
        .and(json_body())
        .and(with_favorites(fav))
        .and(with_db(db))
        .and_then(favorite_handler::add_favorites)
}

fn remove_favorites(
    secret: String,
    fav: Favorites,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "favorites" / "source" / String / "manga" / String)
        .and(warp::delete())
        .and(with_authorization(secret))
        .and(with_favorites(fav))
        .and(with_db(db))
        .and_then(favorite_handler::remove_favorites)
}

fn with_favorites(
    fav: Favorites,
) -> impl Filter<Extract = (Favorites,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || fav.clone())
}

fn json_body() -> impl Filter<Extract = (FavoriteManga,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}
