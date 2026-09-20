extern crate log;

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

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

fn publish_repository(staged: &Path, destination: &Path) -> Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if destination.try_exists()? {
        // Exchange whole directories: readers see either complete generation.
        // The previous generation moves into the temporary directory for cleanup.
        rustix::fs::renameat_with(
            rustix::fs::CWD,
            staged,
            rustix::fs::CWD,
            destination,
            rustix::fs::RenameFlags::EXCHANGE,
        )
        .context("could not atomically replace the generated repository")?;
        return Ok(());
    }

    // On other platforms, replacing a nonempty directory fails without changing
    // it. Do not fall back to a sequence of renames that exposes a partial state.
    std::fs::rename(staged, destination)
        .context("could not atomically publish the generated repository")
}

#[tokio::main]
async fn run() -> Result<()> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        Command::GenerateJson => {
            let target_dir_path = PathBuf::new().join("output").join(TARGET);
            tokio::fs::create_dir_all("output").await?;
            // Keep staging on the destination filesystem for atomic promotion.
            let staging = tempfile::Builder::new()
                .prefix(".tanoshi-index-")
                .tempdir_in("output")?;
            let validation_path = staging.path().join("validation");
            let repository_path = staging.path().join("repository");
            tokio::fs::create_dir(&validation_path).await?;
            tokio::fs::create_dir(&repository_path).await?;

            let mut plugin_names = Vec::new();
            let mut read_dir = tokio::fs::read_dir(&opts.path).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(PLUGIN_EXTENSION) {
                    #[cfg(target_os = "linux")]
                    let name = name.strip_prefix("lib").unwrap_or(&name).to_owned();

                    tokio::fs::copy(entry.path(), validation_path.join(&name)).await?;
                    plugin_names.push(name);
                }
            }

            ensure!(
                !plugin_names.is_empty(),
                "no extensions found in {}; refusing to generate an empty source index",
                opts.path
            );

            let extension_manager = ExtensionManager::new(&validation_path);
            // Server startup tolerates failed plugins; publishing an index must not.
            // Only load this run's inputs, excluding stale output from earlier runs.
            for name in &plugin_names {
                extension_manager.load(name).await.with_context(|| {
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
            for name in plugin_names {
                tokio::fs::copy(validation_path.join(&name), repository_path.join(&name)).await?;
            }
            // Stop workers and discard their private copies before publication.
            for index in &indexes {
                extension_manager.unload(index.source.id).await?;
            }
            tokio::fs::write(repository_path.join("index.json"), json).await?;
            publish_repository(&repository_path, &target_dir_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::publish_repository;
    use std::fs;

    #[test]
    fn publishes_complete_repository_to_new_destination() {
        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("staged");
        let destination = temp.path().join("published");
        fs::create_dir(&staged).unwrap();
        fs::write(staged.join("plugin"), b"new binary").unwrap();
        fs::write(staged.join("index.json"), b"new index").unwrap();

        publish_repository(&staged, &destination).unwrap();

        assert_eq!(fs::read(destination.join("plugin")).unwrap(), b"new binary");
        assert_eq!(
            fs::read(destination.join("index.json")).unwrap(),
            b"new index"
        );
        assert!(!staged.exists());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn replaces_complete_repository_and_removes_stale_plugins() {
        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("staged");
        let destination = temp.path().join("published");
        for directory in [&staged, &destination] {
            fs::create_dir(directory).unwrap();
        }
        fs::write(staged.join("plugin"), b"new binary").unwrap();
        fs::write(staged.join("index.json"), b"new index").unwrap();
        fs::write(destination.join("plugin"), b"old binary").unwrap();
        fs::write(destination.join("index.json"), b"old index").unwrap();
        fs::write(destination.join("obsolete"), b"obsolete binary").unwrap();

        publish_repository(&staged, &destination).unwrap();

        assert_eq!(fs::read(destination.join("plugin")).unwrap(), b"new binary");
        assert_eq!(
            fs::read(destination.join("index.json")).unwrap(),
            b"new index"
        );
        assert!(!destination.join("obsolete").exists());
    }

    #[test]
    fn failed_promotion_preserves_previous_repository() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("published");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("plugin"), b"old binary").unwrap();
        fs::write(destination.join("index.json"), b"old index").unwrap();

        assert!(publish_repository(&temp.path().join("missing"), &destination).is_err());

        assert_eq!(fs::read(destination.join("plugin")).unwrap(), b"old binary");
        assert_eq!(
            fs::read(destination.join("index.json")).unwrap(),
            b"old index"
        );
    }
}
