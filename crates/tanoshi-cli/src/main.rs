extern crate log;

mod data;
mod test;

use clap::{Parser, Subcommand};

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
    Test { file: Option<String> },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let opts: Opts = Opts::parse();

    match opts.subcmd {
        Command::Test { file } => {
            if test::test(file).await.is_err() {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
