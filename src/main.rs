extern crate argon2;

use crate::scraper::mangasee::Mangasee;
use pretty_env_logger;
use std::str::FromStr;
use warp::Filter;

mod auth;
mod favorites;
mod filters;
mod handlers;
mod history;
mod model;
mod scraper;
mod settings;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let db_path = std::env::var("DB_PATH").unwrap_or("./db".to_string());
    let cache_capacity: u64 = u64::from_str(
        std::env::var("DB_CACHE")
            .unwrap_or("100000000".to_string())
            .as_str(),
    )
    .unwrap_or(10_000_000_000);
    let db = sled::Config::default()
        .cache_capacity(cache_capacity)
        .path(db_path)
        .open()
        .unwrap();

    let auth = auth::auth::Auth::new();
    let auth_api = filters::auth::auth::authentication(auth.clone(), db.clone());

    let mangasee = Mangasee::default();
    let mangasee_api = filters::mangasee::mangasee::mangasee(mangasee, auth.clone(), db.clone());

    let fav = favorites::favorites::Favorites::new();
    let fav_api = filters::favorites::favorites::favorites(fav, auth.clone(), db.clone());

    let history = history::history::History::default();
    let history_api = filters::history::history::history(history, auth.clone(), db.clone());

    let settings = settings::settings::Settings::default();
    let settings_api = filters::settings::settings::settings(settings, auth, db);

    let api = auth_api
        .or(fav_api)
        .or(history_api)
        .or(settings_api)
        .or(mangasee_api);

    let static_path = std::env::var("STATIC_FILES_PATH").unwrap_or("./dist".to_string());
    let static_files = warp::fs::dir(static_path);

    let routes = api.or(static_files).with(warp::log("manga"));

    let port = std::env::var("PORT").unwrap_or("80".to_string());
    warp::serve(routes)
        .run(std::net::SocketAddrV4::from_str(format!("0.0.0.0:{}", port).as_str()).unwrap())
        .await;
}
