// use crate::auth::auth::Auth;
// use crate::auth::Claims;
// use crate::filters::settings::settings::auth_handler;
// use crate::filters::with_db;
use crate::filters::{with_authorization, with_db};
use crate::handlers::manga;
use tanoshi::manga::{ GetParams, Params, ImageProxyParam};
use sqlx::postgres::PgPool;
use std::sync::{Arc, Mutex};
use warp::Filter;
use tanoshi::mangadex::MangadexLogin;

pub fn manga(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    list_sources(db.clone())
        .or(list_mangas(secret.clone(), db.clone()))
        .or(get_manga_info(secret.clone(), db.clone()))
        .or(get_chapters(secret.clone(), db.clone()))
        .or(get_pages(db.clone()))
        .or(login(secret.clone(), db.clone()))
        .or(proxy_image())
}

pub fn list_sources(
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "source")
        .and(warp::get())
        .and(with_db(db))
        .and_then(manga::list_sources)
}

pub fn list_mangas(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "source" / i32)
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<Params>())
        .and(with_db(db))
        .and_then(manga::list_mangas)
}

pub fn get_manga_info(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "manga" / i32)
        .and(warp::get())
        .and(with_authorization(secret))
        .and(with_db(db))
        .and_then(manga::get_manga_info)
}

pub fn get_chapters(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "manga" / i32 / "chapter")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<GetParams>())
        .and(with_db(db))
        .and_then(manga::get_chapters)
}

pub fn get_pages(
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "chapter" / i32)
        .and(warp::get())
        .and(warp::query::<GetParams>())
        .and(with_db(db))
        .and_then(manga::get_pages)
}

pub fn proxy_image() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone { 
    warp::path!("api" / "image")
        .and(warp::get())
        .and(warp::query::<ImageProxyParam>())
        .and_then(manga::proxy_image)
}

pub fn login(
    secret: String,
    db: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "source" / "mangadex" / "login")
        .and(warp::post())
        .and(with_authorization(secret))
        .and(json_body())
        .and(with_db(db))
        .and_then(manga::login)
}

fn json_body() -> impl Filter<Extract = (MangadexLogin,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

