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

use anyhow::{anyhow, Result};
use clap::Clap;

use std::sync::{Arc, RwLock};
use warp::Filter;
use config::Config;

#[derive(Clap)]
#[clap(version = "0.13.0")]
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
    let plugin_config = config.plugin_config;
    let plugin_path = config.plugin_path.clone();

    info!("Plugins directory: {}", &plugin_path);

    let extensions = Arc::new(RwLock::new(extension::Extensions::new()));

    for entry in std::fs::read_dir(&plugin_path)?
        .into_iter()
        .filter(move |path| {
            if let Ok(p) = path {
                let ext = p
                    .clone()
                    .path()
                    .extension()
                    .unwrap_or("".as_ref())
                    .to_owned();
                if ext == "so" || ext == "dll" || ext == "dylib" {
                    return true;
                }
            }
            return false;
        })
    {
        let path = entry?.path();
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string()
            .replace("lib", "");
        info!("load plugin from {:?}", path.clone());
        let mut exts = extensions.write().unwrap();
        unsafe {
            match exts.load(path.to_str().unwrap().to_string(), plugin_config.get(&name)) {
                Ok(_) => {}
                Err(e) => error!("not a valid extensions {}", e),
            }
        }
    }

    let serve_static = filters::static_files::static_files();

    let auth = auth::auth::Auth::new(config.database_path.clone());
    let auth_api = filters::auth::authentication(secret.clone(), auth.clone());

    let manga = extension::manga::Manga::new(config.database_path.clone(), extensions.clone());
    let manga_api = filters::manga::manga(secret.clone(), plugin_path.clone(), manga);

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

