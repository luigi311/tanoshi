extern crate log;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tanoshi_vm::{prelude::SourceBus, vm::PLUGIN_EXTENSION};

const TARGET: &str = env!("TARGET");

#[derive(Parser)]
#[clap(version = "0.1.1")]
struct Opts {
    #[clap(short, long, default_value = "./")]
    path: String,
    #[clap(subcommand)]
    subcmd: Command,
}

#[derive(Subcommand)]
enum Command {
    GenerateJson,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let opts: Opts = Opts::parse();

    match opts.subcmd {
        Command::GenerateJson => {
            let target_dir_path = PathBuf::new().join("output").join(TARGET);
            tokio::fs::create_dir_all(&target_dir_path).await?;

            let mut read_dir = tokio::fs::read_dir(&opts.path).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let mut name = format!("{:?}", entry.file_name());
                name.remove(0);
                name.remove(name.len() - 1);

                if name.ends_with(PLUGIN_EXTENSION) {
                    #[cfg(target_os = "linux")]
                    let name = name.replace("lib", "");

                    tokio::fs::copy(entry.path(), &target_dir_path.join(name).as_path()).await?;
                }
            }

            let extension_manager = SourceBus::new(&target_dir_path);
            extension_manager.load_all().await?;
            let source_list = extension_manager.list().await?;

            let json = serde_json::to_string(&source_list)?;
            tokio::fs::write(target_dir_path.join("index").with_extension("json"), json).await?;
        }
    }

    Ok(())
}
