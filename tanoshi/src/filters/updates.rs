use crate::filters::with_authorization;
use crate::handlers::updates;
use crate::handlers::updates::UpdateParam;

use warp::Filter;

pub fn updates(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    get_updates(secret.clone())
}

pub fn get_updates(
    secret: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "updates")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<UpdateParam>())
        .and_then(updates::get_updates)
}
