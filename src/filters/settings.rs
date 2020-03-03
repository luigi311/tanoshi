pub mod settings {
    use crate::auth::auth::Auth;
    use crate::auth::Claims;
    pub use crate::handlers::auth::auth as auth_handler;
    pub use crate::handlers::settings::settings as settings_handler;
    pub use crate::settings::settings::Settings;
    use crate::settings::SettingParams;
    use sled::Db;
    use warp::Filter;

    pub(crate) fn settings(
        settings: Settings,
        auth: Auth,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_settings(settings.clone(), auth.clone(), db.clone())
            .or(set_settings(settings, auth, db))
    }

    fn get_settings(
        settings: Settings,
        auth: Auth,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "settings")
            .and(warp::get())
            .and(with_authorization(auth))
            .and(with_settings(settings))
            .and(with_db(db))
            .and_then(settings_handler::get_settings)
    }

    fn set_settings(
        settings: Settings,
        auth: Auth,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "settings")
            .and(warp::post())
            .and(with_authorization(auth))
            .and(json_body())
            .and(with_settings(settings))
            .and(with_db(db))
            .and_then(settings_handler::set_settings)
    }

    fn with_authorization(
        auth: Auth,
    ) -> impl Filter<Extract = (Claims,), Error = warp::reject::Rejection> + Clone {
        warp::header::header("authorization")
            .map(move |token: String| auth_handler::validate(token, auth.clone()).unwrap())
    }

    fn with_settings(
        settings: Settings,
    ) -> impl Filter<Extract = (Settings,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || settings.clone())
    }

    fn with_db(db: Db) -> impl Filter<Extract = (Db,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || db.clone())
    }

    fn json_body() -> impl Filter<Extract = (SettingParams,), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}
