use crate::filters::with_authorization;
use crate::handlers::updates;
use crate::update::{Update, UpdateParam};

use warp::Filter;

pub fn updates(
    secret: String,
    update: Update,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    get_updates(secret.clone(), update.clone())
}

pub fn get_updates(
    secret: String,
    update: Update,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "updates")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<UpdateParam>())
        .and(with_update(update))
        .and_then(updates::get_updates)
}

fn with_update(
    update: Update,
) -> impl Filter<Extract = (Update,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || update.clone())
}
