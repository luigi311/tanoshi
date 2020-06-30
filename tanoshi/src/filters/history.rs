use crate::filters::with_authorization;
use crate::handlers::history as history_handler;
use crate::history::{History, HistoryParam};

use tanoshi_lib::rest::HistoryRequest;
use warp::Filter;

pub fn history(
    secret: String,
    history: History,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    get_history(secret.clone(), history.clone()).or(add_history(secret.clone(), history.clone()))
}

fn get_history(
    secret: String,
    history: History,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "history")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<HistoryParam>())
        .and(with_history(history))
        .and_then(history_handler::get_history)
}

fn add_history(
    secret: String,
    history: History,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "history")
        .and(warp::post())
        .and(with_authorization(secret))
        .and(json_body())
        .and(with_history(history))
        .and_then(history_handler::add_history)
}

fn json_body() -> impl Filter<Extract = (HistoryRequest,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

fn with_history(
    history: History,
) -> impl Filter<Extract = (History,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || history.clone())
}
