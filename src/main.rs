extern crate argon2;

use crate::scraper::mangasee::Mangasee;
use pretty_env_logger;
use warp::Filter;

mod auth;
mod favorites;
mod filters;
mod handlers;
mod history;
mod scraper;
mod settings;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let db_path = std::env::var("DB_PATH").unwrap_or("./db".to_string());
    let db = sled::open(db_path).unwrap();

    let auth = auth::auth::Auth::new();
    let auth_api = filters::auth::auth::authentication(auth.clone(), db.clone());

    let mangasee = Mangasee::default();
    let mangasee_api = filters::mangasee::mangasee::mangasee(mangasee, db.clone());

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
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}
