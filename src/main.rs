use crate::scraper::Scraping;
use std::env;
use warp::Filter;

mod filters;
mod handlers;
mod scraper;

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "mangas=info")
    }

    let mangasee = Scraping::new("https://mangaseeonline.us");
    let mangadex = Scraping::new("https://mangadex.org");
    let mangasee_api = filters::mangasee::mangasee::mangasee(mangasee);
    let mangadex_api = filters::mangadex::mangadex::mangadex(mangadex);
    let api = mangasee_api.or(mangadex_api);

    let cors = warp::cors()
        .allow_origin("http://localhost:8000")
        .allow_methods(vec!["GET"]);

    let routes = api.with(warp::log("manga")).with(cors);
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
