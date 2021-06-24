#[macro_use]
extern crate log;

mod extension;
mod data;
mod generate;

use clap::{AppSettings, Clap};
use anyhow::Result;
use generate::generate_json;

#[derive(Clap)]
#[clap(version = "0.1.1", author = "Muhammad Fadhlika <fadhlika@gmail.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(version = "0.1.1", author = "Muhammad Fadhlika <fadhlika@gmail.com>")]
    GenerateJson,
}

fn main() -> Result<()> {
    env_logger::init();

    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::GenerateJson => {
            generate_json().unwrap()
        }
    }
    
    Ok(())
}
