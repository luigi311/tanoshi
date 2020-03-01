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

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let auth = auth::auth::Auth::new();
    let auth_api = filters::auth::auth::authentication(auth);

    let mangasee = Mangasee::default();
    let mangasee_api = filters::mangasee::mangasee::mangasee(mangasee);

    let fav = favorites::favorites::Favorites::new();
    let fav_api = filters::favorites::favorites::favorites(fav);

    let history = history::history::History::default();
    let history_api = filters::history::history::history(history);

    let api = auth_api.or(fav_api).or(history_api).or(mangasee_api);

    let static_files = warp::fs::dir("./dist");

    let routes = api.or(static_files).with(warp::log("manga"));
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}
