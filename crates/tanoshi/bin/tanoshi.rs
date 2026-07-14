#[macro_use]
extern crate log;
extern crate argon2;

use clap::Parser;
use futures::future::OptionFuture;
use std::{ffi::OsString, path::PathBuf};

use tanoshi::{application::bootstrap, infrastructure::config::Config};

#[derive(Parser)]
struct Opts {
    /// Path to config file
    #[clap(long)]
    config: Option<String>,
}

fn extension_worker_path() -> Option<PathBuf> {
    let mut args = std::env::args_os();
    let _program = args.next();
    if args.next() != Some(OsString::from("--tanoshi-extension-worker")) {
        return None;
    }
    match (args.next(), args.next(), args.next()) {
        (Some(flag), Some(path), None) if flag == OsString::from("--plugin") => {
            Some(PathBuf::from(path))
        }
        _ => None,
    }
}

fn main() -> Result<(), anyhow::Error> {
    if let Some(plugin_path) = extension_worker_path() {
        return tanoshi_vm::extension::worker::run_worker(plugin_path);
    }

    if std::env::var_os("TANOSHI_EXTENSION_WORKER").is_none()
        && let Ok(executable) = std::env::current_exe()
    {
        // Set this before bootstrap so bundled server deployments can use
        // their own executable as the worker process.
        unsafe { std::env::set_var("TANOSHI_EXTENSION_WORKER", executable) };
    }

    run_server()
}

#[tokio::main]
async fn run_server() -> Result<(), anyhow::Error> {
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        info!("rust_log: {rust_log}");
    } else if let Ok(tanoshi_log) = std::env::var("TANOSHI_LOG") {
        info!("tanoshi_log: {tanoshi_log}");
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe {
            std::env::set_var(
                "RUST_LOG",
                format!("tanoshi={tanoshi_log},tanoshi_vm={tanoshi_log}"),
            );
        };
    }

    env_logger::init();

    let opts: Opts = Opts::parse();
    let config = Config::open(opts.config)?;

    debug!("config: {config:?}");

    let port = config.port;
    let app = bootstrap::bootstrap(config).await?;

    let telegram_bot_fut: OptionFuture<_> = app.telegram_bot.into();
    let server_fut = app.server_builder.serve(([0, 0, 0, 0], port));

    info!("server listening on port {port}");

    tokio::select! {
        _ = server_fut => {
            info!("server shutdown");
        }
        _ = app.update_worker_handle => {
            warn!("update worker quit unexpectedly");
        }
        _ = app.download_worker_handle => {
            warn!("download worker quit unexpectedly");
        }
        Some(()) = telegram_bot_fut => {
            info!("telegram bot shutdown");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("ctrl+c signal received, shutting down");
        }
    }

    info!("closing database...");
    app.pool.close().await;

    Ok(())
}
