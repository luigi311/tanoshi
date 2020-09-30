extern crate argon2;
extern crate libloading as lib;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

mod auth;
mod config;
mod extension;
mod favorites;
mod filters;
mod handlers;
mod history;
mod update;
mod migration;

use anyhow::Result;
use clap::Clap;

use std::sync::{Arc, RwLock};
use warp::Filter;
use config::Config;

#[derive(Clap)]
#[clap(version = "0.14.0")]
struct Opts {
    /// Path to config file
    #[clap(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let opts: Opts = Opts::parse();
    let config = Config::open(opts.config)?;

    if migration::migrate(&config.database_path).await.is_err() {
        log::error!("failed when migrating database...");
    }

    let secret = config.secret;
    let extensions = Arc::new(RwLock::new(extension::Extensions::new()));
    {
        let mut exts = extensions.write().unwrap();
        if exts.initialize(config.plugin_path.clone(), config.plugin_config).is_err() {
            log::error!("error initialize plugin");
        }
    }

    let serve_static = filters::static_files::static_files();

    let auth = auth::auth::Auth::new(config.database_path.clone());
    let auth_api = filters::auth::authentication(secret.clone(), auth.clone());

    let manga = extension::manga::Manga::new(config.database_path.clone(), extensions.clone());
    let manga_api = filters::manga::manga(secret.clone(), config.plugin_path.clone(), manga);

    let fav = favorites::Favorites::new(config.database_path.clone());
    let fav_api = filters::favorites::favorites(secret.clone(), fav);

    let history = history::History::new(config.database_path.clone());
    let history_api = filters::history::history(secret.clone(), history.clone());

    let update = update::Update::new(config.database_path.clone());
    let updates_api = filters::updates::updates(secret.clone(), update.clone());

    let version_check = warp::path!("api" / "version")
        .and(warp::get())
        .map(|| Ok(warp::reply::html(env!("CARGO_PKG_VERSION"))));

    let api = manga_api
        .or(auth_api)
        .or(fav_api)
        .or(history_api)
        .or(updates_api)
        .or(version_check)
        .recover(filters::handle_rejection);

    let routes = api.or(serve_static).with(warp::log("manga"));

    warp::serve(routes).run(([0, 0, 0, 0], config.port)).await;

    return Ok(());
}

