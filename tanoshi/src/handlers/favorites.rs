use crate::auth::Claims;
use crate::favorites::{favorites::Favorites};
use tanoshi_lib::manga::FavoriteManga;
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

pub async fn get_favorites(
    claim: Claims,
    fav: Favorites,
    db: PgPool,
) -> Result<impl warp::Reply, Infallible> {
    let res = fav.get_favorites(claim.sub, db).await;
    Ok(warp::reply::json(&res))
}

pub async fn add_favorites(
    manga_id: i32,
    claim: Claims,
    fav: Favorites,
    db: PgPool,
) -> Result<impl warp::Reply, Infallible> {
    let res = fav.add_favorite(claim.sub, manga_id, db).await;
    Ok(warp::reply::json(&res))
}

pub async fn remove_favorites(
    manga_id: i32,
    claim: Claims,
    fav: Favorites,
    db: PgPool,
) -> Result<impl warp::Reply, Infallible> {
    let res = fav.remove_favorites(claim.sub, manga_id, db).await;
    Ok(warp::reply::json(&res))
}
