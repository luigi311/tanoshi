use crate::auth::Claims;
use crate::update::{Update, UpdateParam};
use warp::Rejection;

pub async fn get_updates(
    claim: Claims,
    param: UpdateParam,
    update: Update,
) -> Result<impl warp::Reply, Rejection> {
    update.get_updates(claim, param).await
}
