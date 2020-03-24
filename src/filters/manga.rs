// use crate::auth::auth::Auth;
// use crate::auth::Claims;
// use crate::filters::settings::settings::auth_handler;
// use crate::filters::with_db;
use crate::filters::{with_authorization, with_db};
use crate::handlers::manga;
use crate::scraper::{mangasee::Mangasee, GetParams, Params};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use warp::Filter;

pub fn manga(
    secret: String,
    db: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    list_mangas(db.clone())
        .or(get_manga_info(db.clone()))
        .or(get_chapters(secret, db.clone()))
        .or(get_pages(db.clone()))
}

pub fn list_mangas(
    db: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "source" / String)
        .and(warp::get())
        .and(warp::query::<Params>())
        .and(with_db(db))
        .and_then(manga::list_mangas)
}

pub fn get_manga_info(
    db: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "source" / String / "manga" / String)
        .and(warp::get())
        .and(with_db(db))
        .and_then(manga::get_manga_info)
}

pub fn get_chapters(
    secret: String,
    db: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "source" / String / "manga" / String / "chapter")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<GetParams>())
        .and(with_db(db))
        .and_then(manga::get_chapters)
}

pub fn get_pages(
    db: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "source" / String / "manga" / String / "chapter" / String)
        .and(warp::get())
        .and(warp::query::<GetParams>())
        .and(with_db(db))
        .and_then(manga::get_pages)
}
