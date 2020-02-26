extern crate argon2;

use crate::scraper::mangasee::Mangasee;
use pretty_env_logger;
use warp::Filter;

mod auth;
mod filters;
mod handlers;
mod scraper;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let tree = Box::from(sled::open("./tanoshi.db").unwrap());

    let auth = auth::auth::Auth::new(tree);
    let auth_api = filters::auth::auth::authentication(auth);

    let mangasee = Mangasee::default();
    let mangasee_api = filters::mangasee::mangasee::mangasee(mangasee);
    let api = auth_api.or(mangasee_api);
    let static_files = warp::fs::dir("./dist");

    let cors = warp::cors()
        .allow_origin("http://localhost:8000")
        .allow_methods(vec!["GET"]);

    let routes = api.or(static_files).with(cors).with(warp::log("manga"));
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
