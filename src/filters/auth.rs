pub mod auth {
    use crate::auth::auth::Auth;
    use crate::auth::User;
    use crate::filters::with_db;
    use crate::handlers::auth::auth as auth_handler;
    use sled::Tree;
    use warp::Filter;

    pub fn authentication(
        auth: Auth,
        db: Tree,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        login(auth.clone(), db.clone()).or(register(auth, db))
    }

    pub fn login(
        auth: Auth,
        db: Tree,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "login")
            .and(warp::post())
            .and(json_body())
            .and(with_auth(auth))
            .and(with_db(db))
            .and_then(auth_handler::login)
    }

    pub fn register(
        auth: Auth,
        db: Tree,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "register")
            .and(warp::post())
            .and(json_body())
            .and(with_auth(auth))
            .and(with_db(db))
            .and_then(auth_handler::register)
    }

    fn with_auth(
        auth: Auth,
    ) -> impl Filter<Extract = (Auth,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || auth.clone())
    }

    fn json_body() -> impl Filter<Extract = (User,), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}
