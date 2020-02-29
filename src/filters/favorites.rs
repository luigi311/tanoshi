pub mod favorites {
    use crate::favorites::favorites::Favorites;
    use crate::favorites::FavoriteManga;
    use crate::handlers::auth::auth as auth_handler;
    use crate::handlers::favorites::favorites as favorite_handler;
    use warp::Filter;

    pub(crate) fn favorites(
        fav: Favorites,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_favorites(fav.clone()).or(add_favorites(fav))
    }

    fn get_favorites(
        fav: Favorites,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "favorites")
            .and(warp::get())
            .and(auth_handler::validate())
            .and(with_favorites(fav))
            .and_then(favorite_handler::get_favorites)
    }

    fn add_favorites(
        fav: Favorites,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "favorites")
            .and(warp::post())
            .and(auth_handler::validate())
            .and(json_body())
            .and(with_favorites(fav))
            .and_then(favorite_handler::add_favorites)
    }

    fn with_favorites(
        fav: Favorites,
    ) -> impl Filter<Extract = (Favorites,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || fav.clone())
    }

    fn json_body() -> impl Filter<Extract = (FavoriteManga,), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}
