pub mod favorites {
    use crate::auth::auth::Auth;
    use crate::auth::Claims;
    use crate::favorites::favorites::Favorites;
    use crate::favorites::FavoriteManga;
    use crate::filters::with_db;
    use crate::handlers::auth::auth as auth_handler;
    use crate::handlers::favorites::favorites as favorite_handler;
    use sled::Db;
    use warp::Filter;

    pub(crate) fn favorites(
        fav: Favorites,
        auth: Auth,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_favorites(fav.clone(), auth.clone(), db.clone())
            .or(add_favorites(fav.clone(), auth.clone(), db.clone()))
            .or(remove_favorites(fav, auth.clone(), db))
    }

    fn get_favorites(
        fav: Favorites,
        auth: Auth,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "favorites")
            .and(warp::get())
            .and(with_authorization(auth))
            .and(with_favorites(fav))
            .and(with_db(db))
            .and_then(favorite_handler::get_favorites)
    }

    fn add_favorites(
        fav: Favorites,
        auth: Auth,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "favorites")
            .and(warp::post())
            .and(with_authorization(auth))
            .and(json_body())
            .and(with_favorites(fav))
            .and(with_db(db))
            .and_then(favorite_handler::add_favorites)
    }

    fn remove_favorites(
        fav: Favorites,
        auth: Auth,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "favorites")
            .and(warp::delete())
            .and(with_authorization(auth))
            .and(json_body())
            .and(with_favorites(fav))
            .and(with_db(db))
            .and_then(favorite_handler::remove_favorites)
    }

    fn with_authorization(
        auth: Auth,
    ) -> impl Filter<Extract = (Claims,), Error = warp::reject::Rejection> + Clone {
        warp::header::header("authorization").map(move |token: String| {
            let claim = auth_handler::validate(token, auth.clone());
            claim.unwrap_or(Claims::default())
        })
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
