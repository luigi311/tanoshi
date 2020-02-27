pub mod favorites {
    use crate::favorites::favorites::Favorites;
    use crate::handlers::auth::auth as auth_handler;
    use crate::handlers::favorites::favorites as favorite_handler;
    use crate::scraper::{Chapter, Manga};
    use warp::Filter;

    pub(crate) fn favorites(
        fav: Favorites,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_favorites(fav)
    }

    fn get_favorites(
        fav: Favorites,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "favorites")
            .and(warp::get())
            .and(with_authorization())
            .and(with_favorites(fav))
            .and_then(favorite_handler::get_favorites)
    }

    fn with_authorization() -> impl Filter<Extract = (String,), Error = warp::Rejection> + Clone {
        warp::any().and(
            warp::header::header("authorization").map(|token: String| -> String {
                match auth_handler::validate(token) {
                    Some(username) => username,
                    None => "".to_string(),
                    _ => "".to_string(),
                }
            }),
        )
    }

    fn with_favorites(
        fav: Favorites,
    ) -> impl Filter<Extract = (Favorites,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || fav.clone())
    }
}
