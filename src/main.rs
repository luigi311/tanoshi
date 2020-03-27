extern crate argon2;

use std::str::FromStr;

use pretty_env_logger;
use warp::Filter;

use crate::scraper::mangasee::Mangasee;
use std::sync::{Arc, Mutex};

mod auth;
mod favorites;
mod filters;
mod handlers;
mod model;
mod scraper;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let secret = std::env::var("TOKEN_SECRET_KEY").unwrap();
    let db_path = std::env::var("DB_PATH").unwrap_or("./tanoshi.db".to_string());

    let conn = rusqlite::Connection::open(db_path).unwrap();
    let conn = Arc::new(Mutex::new(conn));

    let auth_api = filters::auth::authentication(secret.clone(), conn.clone());
    let manga_api = filters::manga::manga(secret.clone(), conn.clone());

    let fav = favorites::Favorites::new();
    let fav_api = filters::favorites::favorites(secret.clone(), fav, conn.clone());

    let history_api = filters::history::history(secret.clone(), conn.clone());

    let updates_api = filters::updates::updates(secret.clone(), conn.clone());

    let api = manga_api
        .or(auth_api)
        .or(fav_api)
        .or(history_api)
        .or(updates_api);

    let static_path = std::env::var("STATIC_FILES_PATH").unwrap_or("./dist".to_string());
    let static_files = warp::fs::dir(static_path);

    let routes = api.or(static_files).with(warp::log("manga"));

    let port = std::env::var("PORT").unwrap_or("80".to_string());
    warp::serve(routes)
        .run(std::net::SocketAddrV4::from_str(format!("0.0.0.0:{}", port).as_str()).unwrap())
        .await;
}
