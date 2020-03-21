pub mod history {
    use crate::auth::auth::Auth;
    use crate::auth::Claims;
    use crate::filters::{with_authorization, with_db};
    use crate::handlers::auth::auth as auth_handler;
    use crate::handlers::history::history as history_handler;
    use crate::history::{history::History, HistoryRequest};
    use sled::Tree;
    use warp::Filter;

    pub fn history(
        history: History,
        auth: Auth,
        library_tree: Tree,
        scraper_tree: Tree,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_history(history.clone(), auth.clone(), library_tree.clone()).or(add_history(
            history,
            auth,
            library_tree,
            scraper_tree,
        ))
    }

    fn get_history(
        history: History,
        auth: Auth,
        db: Tree,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "history" / "source" / String / "manga" / String)
            .and(warp::get())
            .and(with_authorization(auth))
            .and(with_history(history))
            .and(with_db(db))
            .and_then(history_handler::get_history)
    }

    fn add_history(
        history: History,
        auth: Auth,
        library_tree: Tree,
        scraper_tree: Tree,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "history")
            .and(warp::post())
            .and(with_authorization(auth))
            .and(json_body())
            .and(with_history(history))
            .and(with_db(library_tree))
            .and(with_db(scraper_tree))
            .and_then(history_handler::add_history)
    }

    fn with_history(
        history: History,
    ) -> impl Filter<Extract = (History,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || history.clone())
    }

    fn json_body() -> impl Filter<Extract = (HistoryRequest,), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}
