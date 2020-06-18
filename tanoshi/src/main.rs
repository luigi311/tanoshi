extern crate argon2;
extern crate libloading as lib;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use anyhow::Result;
use clap::Clap;
use rust_embed::RustEmbed;

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use tanoshi_lib::extensions::Extension;
use tokio::sync::RwLock;
use warp::{http::header::HeaderValue, path::Tail, reply::Response, Filter, Rejection, Reply};

mod auth;
mod extension;
mod favorites;
mod filters;
mod handlers;
mod history;
mod update;
mod worker;

#[derive(RustEmbed)]
#[folder = "../tanoshi-web/dist/"]
struct Asset;

#[derive(serde::Deserialize)]
struct Config {
    pub port: Option<String>,
    pub database_path: String,
    pub secret: String,
    pub cache_ttl: u64,
    pub plugin_path: Option<String>,
    pub plugin_config: Option<BTreeMap<String, serde_yaml::Value>>,
}

#[derive(Clap)]
struct Opts {
    /// Create inital admin user account
    #[clap(long)]
    create_admin: bool,
    /// Path to config file
    #[clap(long, default_value = "~/config/tanoshi/config.yml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let opts: Opts = Opts::parse();

    let config: Config = serde_yaml::from_slice(&std::fs::read(opts.config)?)?;

    if opts.create_admin {
        use auth::User;
        let user = User {
            username: "admin".to_string(),
            password: Some("admin".to_string()),
            role: "ADMIN".to_string(),
        };
        //auth::auth::Auth::register(user).await;
        return Ok(());
    }

    let secret = config.secret;
    let plugin_config = config.plugin_config.unwrap_or(BTreeMap::new());

    let extensions = Arc::new(RwLock::new(extension::Extensions::new()));
    for entry in std::fs::read_dir(
        config
            .plugin_path
            .unwrap_or("~/.tanoshi/plugins".to_string()),
    )?
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
    }) {
        let path = entry?.path();
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string()
            .replace("lib", "");
        info!("{}", name.clone());
        info!("load plugin from {:?}", path.clone());
        let config = plugin_config.get(&name);
        let mut exts = extensions.write().await;
        unsafe {
            match exts.load(path, config) {
                Ok(_) => {}
                Err(e) => error!("not a valid extensions {}", e),
            }
        }
    }

    let mut update_worker = worker::Worker::new();
    update_worker.remove_cache(config.cache_ttl);

    let static_files = warp::get().and(warp::path::tail()).and_then(serve);
    let index = warp::get().and_then(serve_index);

    let static_files = static_files.or(index);

    let auth = auth::auth::Auth::new(config.database_path.clone());
    let auth_api = filters::auth::authentication(secret.clone(), auth.clone());

    let manga = extension::manga::Manga::new(config.database_path.clone());
    let manga_api = filters::manga::manga(secret.clone(), extensions, manga);

    let fav = favorites::Favorites::new(config.database_path.clone());
    let fav_api = filters::favorites::favorites(secret.clone(), fav);

    let history = history::History::new(config.database_path.clone());
    let history_api = filters::history::history(secret.clone(), history.clone());

    let update = update::Update::new(config.database_path.clone());
    let updates_api = filters::updates::updates(secret.clone(), update.clone());

    let api = manga_api
        .or(auth_api)
        .or(fav_api)
        .or(history_api)
        .or(updates_api)
        .recover(filters::handle_rejection);

    let routes = api.or(static_files).with(warp::log("manga"));

    let port = config.port.unwrap_or("80".to_string());
    warp::serve(routes)
        .run(std::net::SocketAddrV4::from_str(format!("0.0.0.0:{}", port).as_str()).unwrap())
        .await;

    return Ok(());
}

async fn serve_index() -> Result<impl Reply, Rejection> {
    serve_impl("index.html")
}

async fn serve(path: Tail) -> Result<impl Reply, Rejection> {
    serve_impl(path.as_str())
}

fn serve_impl(path: &str) -> Result<impl Reply, Rejection> {
    let asset = Asset::get(path).ok_or_else(warp::reject::not_found)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let mut res = Response::new(asset.into());
    res.headers_mut().insert(
        "content-type",
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );
    Ok(res)
}
