extern crate argon2;
extern crate libloading as lib;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

mod auth;
mod bot;
mod config;
mod extension;
mod favorites;
mod filters;
mod handlers;
mod history;
mod update;
mod worker;

use anyhow::{anyhow, Result};
use clap::Clap;
use rust_embed::RustEmbed;

use std::sync::{Arc, RwLock};
use warp::{http::header::HeaderValue, path::Tail, reply::Response, Filter, Rejection, Reply};

use config::Config;

lazy_static! {
    static ref QUERIES: Vec<&'static str> = vec![
        include_str!("../migration/1.sql"),
        include_str!("../migration/2.sql"),
        include_str!("../migration/3.sql"),
        include_str!("../migration/4.sql"),
    ];
}

#[derive(RustEmbed)]
#[folder = "../tanoshi-web/dist/"]
struct Asset;

#[derive(Clap)]
#[clap(version = "0.8.5")]
struct Opts {
    /// Path to config file
    #[clap(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let opts: Opts = Opts::parse();

    let slice = match opts.config {
        Some(path) => {
            info!("Load config from {}", path.clone());
            match std::fs::read(path) {
                Ok(slice) => Some(slice),
                Err(_e) => None,
            }
        }
        None => {
            let path = dirs::home_dir()
                .unwrap()
                .join(".tanoshi")
                .join("config.yml");
            match std::fs::read(path.clone()) {
                Ok(slice) => Some(slice),
                Err(_e) => None,
            }
        }
    };

    let config: Config = match slice {
        Some(s) => match serde_yaml::from_slice(&s) {
            Ok(config) => config,
            Err(e) => return Err(anyhow!("error parsing config: {}", e)),
        },
        None => {
            let cfg = Config::default();
            let path = dirs::home_dir().unwrap().join(".tanoshi");
            let _ = std::fs::create_dir_all(path.clone());
            let path = path.join("config.yml");
            let _ = std::fs::write(&path, serde_yaml::to_string(&cfg).unwrap());
            info!("Write default config at {:?}", path);
            cfg
        }
    };

    {
        let conn = match rusqlite::Connection::open(config.database_path.clone()) {
            Ok(conn) => conn,
            Err(e) => {
                return Err(anyhow!("failed open database file: {}", e));
            }
        };

        let user_version: i32 = conn
            .pragma_query_value(Some(rusqlite::DatabaseName::Main), "user_version", |row| {
                row.get(0)
            })
            .unwrap_or(0);
        info!("Schema version {}", user_version);

        if QUERIES.len() > user_version as usize {
            info!("Schema version mismatch");
            for (i, query) in QUERIES.iter().enumerate() {
                if i + 1 > user_version as usize {
                    info!("Migrating {}", i + 1);
                    if let Err(e) = conn.execute_batch(query) {
                        return Err(anyhow!("failed: {}", e));
                    }
                }
            }

            if user_version == 0 {
                let auth = auth::auth::Auth::new(config.database_path.clone());
                auth.register(auth::User {
                    username: "admin".to_string(),
                    password: Some("admin".to_string()),
                    role: "ADMIN".to_string(),
                    telegram_chat_id: None,
                })
                .await;
            }

            if let Err(e) = conn.pragma_update(
                Some(rusqlite::DatabaseName::Main),
                "user_version",
                &(QUERIES.len() as i32),
            ) {
                return Err(anyhow!("error set PRAGMA user_version: {}", e));
            }
        }
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
            match exts.load(path, plugin_config.get(&name)) {
                Ok(_) => {}
                Err(e) => error!("not a valid extensions {}", e),
            }
        }
    }

    let bot = match config.telegram_token.clone() {
        Some(token) => Some(bot::Bot::new(token)),
        None => None,
    };

    let update_worker = worker::Worker::new();
    update_worker.remove_cache(config.cache_ttl);
    update_worker.check_update(
        config.update_interval,
        config.database_path.clone(),
        config.base_url.unwrap_or("".to_string()),
        extensions.clone(),
        bot.clone().map(|b| b.get_pub()),
    );

    if let Some(bot) = bot.clone() {
        bot.start();
    }

    let static_files = warp::get().and(warp::path::tail()).and_then(serve);
    let index = warp::get().and_then(serve_index);

    let static_files = static_files.or(index);

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

    let version_check = warp::path!("version")
        .and(warp::get())
        .map(|| Ok(warp::reply::html(env!("CARGO_PKG_VERSION"))));

    let api = manga_api
        .or(auth_api)
        .or(fav_api)
        .or(history_api)
        .or(updates_api)
        .or(version_check)
        .recover(filters::handle_rejection);

    let routes = api.or(static_files).with(warp::log("manga"));

    warp::serve(routes).run(([0, 0, 0, 0], config.port)).await;

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
