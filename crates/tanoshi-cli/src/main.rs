extern crate log;

mod data;
mod generate;
mod run;

use clap::{Parser, Subcommand};
use tanoshi_vm::extension::SourceManager;

#[derive(Parser)]
#[clap(version = "0.1.1", author = "Muhammad Fadhlika <fadhlika@gmail.com>")]
struct Opts {
    #[clap(short, long, default_value = "./")]
    path: String,
    #[clap(subcommand)]
    subcmd: Command,
}

#[derive(Subcommand)]
enum Command {
    GenerateJson,
    Run { name: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let opts: Opts = Opts::parse();

    let manager = SourceManager::new(&opts.path);

    // let mut read_dir = tokio::fs::read_dir(&opts.path).await?;
    // while let Some(entry) = read_dir.next_entry().await? {
    //     let name = format!("{:?}", entry.file_name());
    //     manager.load(&name[1..name.len() - 5])?;
    // }

    match opts.subcmd {
        Command::GenerateJson => todo!(),
        Command::Run { name } => run::run(manager, &name).await?,
    }

    Ok(())
}
