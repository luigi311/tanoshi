extern crate argon2;

use std::str::FromStr;

use pretty_env_logger;
use warp::Filter;

use crate::scraper::mangasee::Mangasee;

mod auth;
mod favorites;
mod filters;
mod handlers;
mod history;
mod model;
mod scraper;
mod settings;

mod utils;

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

    let user_tree = db.open_tree("user").expect("failed to open tree");

    let library_tree = db.open_tree("library").expect("failed to open tree");

    let scraper_tree = db.open_tree("scraper").expect("failed to open tree");

    library_tree.set_merge_operator(utils::merge_library);

    scraper_tree
        .insert("mangasee", "https://mangaseeonline.us")
        .expect("failed to insert sorce");

    let auth = auth::auth::Auth::new();
    let auth_api = filters::auth::auth::authentication(auth.clone(), user_tree.clone());

    let manga_api = filters::manga::manga::manga(scraper_tree.clone());

    let fav = favorites::favorites::Favorites::new();
    let fav_api = filters::favorites::favorites::favorites(
        fav,
        auth.clone(),
        library_tree.clone(),
        scraper_tree.clone(),
    );

    let history = history::history::History::default();
    let history_api = filters::history::history::history(
        history,
        auth.clone(),
        library_tree.clone(),
        scraper_tree.clone(),
    );

    let settings = settings::settings::Settings::default();
    let settings_api = filters::settings::settings::settings(settings, auth, user_tree);

    let api = auth_api
        .or(fav_api)
        .or(history_api)
        .or(settings_api)
        .or(manga_api);

    let static_path = std::env::var("STATIC_FILES_PATH").unwrap_or("./dist".to_string());
    let static_files = warp::fs::dir(static_path);

    let routes = api.or(static_files).with(warp::log("manga"));

    let port = std::env::var("PORT").unwrap_or("80".to_string());
    warp::serve(routes)
        .run(std::net::SocketAddrV4::from_str(format!("0.0.0.0:{}", port).as_str()).unwrap())
        .await;
}
