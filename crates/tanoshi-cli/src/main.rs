extern crate log;

use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use serde::Serialize;
use tanoshi_lib::prelude::SourceInfo;
use tanoshi_vm::{PLUGIN_EXTENSION, prelude::ExtensionManager};

const TARGET: &str = env!("TARGET");

#[derive(Parser)]
#[clap(version, about)]
struct Opts {
    #[clap(short, long, default_value = "./")]
    path: String,
    #[clap(subcommand)]
    subcmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate index.json
    GenerateJson,
}

#[derive(Debug, Serialize)]
struct SourceIndex {
    #[serde(flatten)]
    source: SourceInfo,
    rustc_version: String,
    lib_version: String,
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

fn main() -> Result<()> {
    if let Some(plugin_path) = extension_worker_path() {
        return tanoshi_vm::extension::worker::run_worker(plugin_path);
    }

    if std::env::var_os("TANOSHI_EXTENSION_WORKER").is_none() {
        let executable = std::env::current_exe()?;
        // SAFETY: Configure the worker before starting the runtime or any threads.
        unsafe { std::env::set_var("TANOSHI_EXTENSION_WORKER", executable) };
    }

    env_logger::init();
    run()
}

#[tokio::main]
async fn run() -> Result<()> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        Command::GenerateJson => {
            let target_dir_path = PathBuf::new().join("output").join(TARGET);
            tokio::fs::create_dir_all(&target_dir_path).await?;

            let mut plugin_names = Vec::new();
            let mut read_dir = tokio::fs::read_dir(&opts.path).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(PLUGIN_EXTENSION) {
                    #[cfg(target_os = "linux")]
                    let name = name.strip_prefix("lib").unwrap_or(&name).to_owned();

                    tokio::fs::copy(entry.path(), target_dir_path.join(&name)).await?;
                    plugin_names.push(name);
                }
            }

            ensure!(
                !plugin_names.is_empty(),
                "no extensions found in {}; refusing to generate an empty source index",
                opts.path
            );

            let extension_manager = ExtensionManager::new(&target_dir_path);
            // Server startup tolerates failed plugins; publishing an index must not.
            // Only load this run's inputs, excluding stale output from earlier runs.
            for name in plugin_names {
                extension_manager.load(&name).await.with_context(|| {
                    format!("failed to load {name}; source index was not written")
                })?;
            }
            let source_list = extension_manager.list().await?;

            let mut indexes = vec![];
            for source in source_list {
                let (rustc_version, lib_version) = extension_manager.get_version(source.id)?;
                indexes.push(SourceIndex {
                    source,
                    rustc_version,
                    lib_version,
                });
            }

            let json = serde_json::to_string(&indexes)?;
            tokio::fs::write(target_dir_path.join("index").with_extension("json"), json).await?;
        }
    }

    Ok(())
}
