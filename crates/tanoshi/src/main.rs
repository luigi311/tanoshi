#[macro_use]
extern crate log;
extern crate argon2;

#[cfg(feature = "embed")]
mod assets;
mod catalogue;
mod config;
mod db;
mod downloads;
mod guard;
mod library;
mod local;
mod notification;
mod notifier;
mod proxy;
mod schema;
mod server;
mod status;
mod user;
mod utils;
mod worker;

use crate::{
    config::{Config, GLOBAL_CONFIG},
    notifier::pushover::Pushover,
};
use clap::Parser;
use futures::future::OptionFuture;
use tanoshi_vm::{extension::SourceBus, prelude::Source};

use teloxide::prelude::RequesterExt;

#[derive(Parser)]
struct Opts {
    /// Path to config file
    #[clap(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        info!("rust_log: {}", rust_log);
    } else if let Ok(tanoshi_log) = std::env::var("TANOSHI_LOG") {
        info!("tanoshi_log: {}", tanoshi_log);
        std::env::set_var(
            "RUST_LOG",
            format!("tanoshi={},tanoshi_vm={}", tanoshi_log, tanoshi_log),
        );
    }

    env_logger::init();

    let opts: Opts = Opts::parse();
    let config =
        GLOBAL_CONFIG.get_or_init(|| Config::open(opts.config).expect("failed to init config"));

    debug!("config: {:?}", config);

    let pool = db::establish_connection(&config.database_path).await?;
    let mangadb = db::MangaDatabase::new(pool.clone());
    let userdb = db::UserDatabase::new(pool.clone());

    let extension_manager = SourceBus::new(&config.plugin_path);

    extension_manager.load_all().await?;

    match &config.local_path {
        config::LocalFolders::Single(local_path) => {
            extension_manager
                .insert(Source::from(Box::new(local::Local::new(
                    10000,
                    "Local".to_string(),
                    local_path,
                ))))
                .await?;
        }
        config::LocalFolders::Multiple(local_paths) => {
            for (index, local_path) in local_paths.iter().enumerate() {
                // source id starts from 10000
                let index = index + 10000;
                extension_manager
                    .insert(Source::from(Box::new(local::Local::new(
                        index as i64,
                        local_path.name.clone(),
                        &local_path.path,
                    ))))
                    .await?;
            }
        }
    }

    let mut notifier_builder = notifier::Builder::new(userdb.clone());

    let mut telegram_bot_fut: OptionFuture<_> = None.into();
    if let Some(telegram_config) = config.telegram.clone() {
        let bot = teloxide::Bot::new(telegram_config.token)
            .auto_send()
            .parse_mode(teloxide::types::ParseMode::Html);
        telegram_bot_fut = Some(notifier::telegram::run(bot.clone())).into();
        notifier_builder = notifier_builder.telegram(bot);
    }

    if let Some(pushover_cfg) = config.pushover.as_ref() {
        notifier_builder =
            notifier_builder.pushover(Pushover::new(pushover_cfg.application_key.clone()));
    }

    let notifier = notifier_builder.finish();

    let (download_tx, download_worker_handle) = worker::downloads::start(
        &config.download_path,
        mangadb.clone(),
        extension_manager.clone(),
        notifier.clone(),
    );

    let update_worker_handle = worker::updates::start(
        config.update_interval,
        mangadb.clone(),
        extension_manager.clone(),
        download_tx.clone(),
        notifier.clone(),
    );

    let schema = schema::build(userdb, mangadb, extension_manager, download_tx, notifier);

    let app = server::init_app(config, schema);
    let server_fut = server::serve("0.0.0.0", config.port, app);

    tokio::select! {
        _ = server_fut => {
            info!("server shutdown");
        }
        _ = update_worker_handle => {
            info!("update worker quit");
        }
        _ = download_worker_handle => {
            info!("download worker quit");
        }
        Some(_) = telegram_bot_fut => {
            info!("worker shutdown");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("ctrl+c signal");
        }
    }

    info!("closing database...");
    pool.close().await;

    Ok(())
}
