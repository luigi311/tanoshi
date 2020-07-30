use crate::favorites::favorites::Favorites;
use crate::filters::with_authorization;
use crate::handlers::favorites as favorite_handler;

use warp::Filter;
use tanoshi_lib::manga::Params;

pub fn favorites(
    secret: String,
    fav: Favorites,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    get_favorites(secret.clone(), fav.clone())
        .or(add_favorites(secret.clone(), fav.clone()))
        .or(remove_favorites(secret, fav))
}

fn get_favorites(
    secret: String,
    fav: Favorites,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "favorites")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<Params>())
        .and(with_favorites(fav))
        .and_then(favorite_handler::get_favorites)
}

fn add_favorites(
    secret: String,
    fav: Favorites,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "favorites" / "manga" / i32)
        .and(warp::post())
        .and(with_authorization(secret))
        .and(with_favorites(fav))
        .and_then(favorite_handler::add_favorites)
}

fn remove_favorites(
    secret: String,
    fav: Favorites,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "favorites" / "manga" / i32)
        .and(warp::delete())
        .and(with_authorization(secret))
        .and(with_favorites(fav))
        .and_then(favorite_handler::remove_favorites)
}

fn with_favorites(
    fav: Favorites,
) -> impl Filter<Extract = (Favorites,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || fav.clone())
}
