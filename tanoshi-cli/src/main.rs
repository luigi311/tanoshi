#[macro_use]
extern crate log;

mod extension;
mod data;
mod generate;

use clap::{AppSettings, Clap};
use anyhow::Result;

#[derive(Clap)]
#[clap(version = "0.1.1", author = "Muhammad Fadhlika <fadhlika@gmail.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    GenerateJson,
    TestExtension(TestExtension)
}

#[derive(Clap)]
struct TestExtension {
    path: String
}

fn main() -> Result<()> {
    env_logger::init();

    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::GenerateJson => {
            generate::generate_json().unwrap();
        }
        SubCommand::TestExtension(test) => {
            extension::test(test.path);
        },
    }
    
    Ok(())
}
