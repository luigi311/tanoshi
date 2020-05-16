use crate::favorites::favorites::Favorites;
use tanoshi::manga::FavoriteManga;
use crate::filters::{with_authorization, with_db};
use crate::handlers::favorites as favorite_handler;
use sqlx::postgres::PgPool;
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
    warp::path!("api" / "favorites" / "manga" / i32)
        .and(warp::post())
        .and(with_authorization(secret))
        .and(with_favorites(fav))
        .and(with_db(db))
        .and_then(favorite_handler::add_favorites)
}

fn remove_favorites(
    secret: String,
    fav: Favorites,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "favorites" / "manga" / i32)
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