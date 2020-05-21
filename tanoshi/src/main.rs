extern crate argon2;
extern crate libloading as lib;
extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use sqlx::postgres::PgPool;
use warp::Filter;
use anyhow::Result;

mod auth;
mod favorites;
mod filters;
mod handlers;
mod extension;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let secret = std::env::var("TOKEN_SECRET_KEY").unwrap();
    let static_path = std::env::var("STATIC_FILES_PATH").unwrap_or("./dist".to_string());
    let plugin_path = std::env::var("PLUGIN_PATH").unwrap_or("./plugins".to_string());
    let manga_path = std::env::var("MANGA_PATH").unwrap_or("./manga".to_string());

    let mut extensions = Arc::new(RwLock::new(extension::Extensions::new()));
    for entry in std::fs::read_dir(plugin_path)? {
        let entry = entry?;
        let path = entry.path();
        let ext = path.extension().unwrap_or("".as_ref()).to_str().unwrap_or("");
        if ext == "so" || ext == "dll" || ext == "dylib" {
            info!("load plugin from {:?}", path.clone());
            let mut exts = extensions.write().await;
            unsafe {
                exts.load(path).expect("not a valid extension");
            }
        }
    }
    let exts = extensions.clone();
    let exts = exts.read().await;
    info!("there are {} plugins", exts.extensions().len());

    let static_files = warp::fs::dir(static_path);

    let pool = PgPool::builder()
        .max_size(5) // maximum number of connections in the pool
        .build(std::env::var("DATABASE_URL").unwrap().as_str())
        .await?;

    let auth_api = filters::auth::authentication(secret.clone(), pool.clone());
    let manga_api = filters::manga::manga(secret.clone(), extensions, pool.clone());

    let fav = favorites::Favorites::new();
    let fav_api = filters::favorites::favorites(secret.clone(), fav, pool.clone());

    let history_api = filters::history::history(secret.clone(), pool.clone());

    let updates_api = filters::updates::updates(secret.clone(), pool.clone());

    let api = manga_api
        .or(auth_api)
        .or(fav_api)
        .or(history_api)
        .or(updates_api)
        .or(static_files)
        .recover(filters::handle_rejection);

    let routes = api.with(warp::log("manga"));

    let port = std::env::var("PORT").unwrap_or("80".to_string());
    warp::serve(routes)
        .run(std::net::SocketAddrV4::from_str(format!("0.0.0.0:{}", port).as_str()).unwrap())
        .await;
        Ok(())
}
