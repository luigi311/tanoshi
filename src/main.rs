extern crate argon2;

use crate::scraper::Scraping;
use pretty_env_logger;
use std::env;
use warp::Filter;

mod filters;
mod handlers;
mod scraper;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mangasee = Scraping::new("https://mangaseeonline.us");
    let mangasee_api = filters::mangasee::mangasee::mangasee(mangasee);
    let api = mangasee_api;
    let static_files = warp::fs::dir("./dist");

    let cors = warp::cors()
        .allow_origin("http://localhost:8000")
        .allow_methods(vec!["GET"]);

    let routes = api.or(static_files).with(cors).with(warp::log("manga"));
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
