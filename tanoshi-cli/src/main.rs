#[macro_use]
extern crate log;

mod data;
mod generate;
mod test;

use clap::{AppSettings, Clap};
use tanoshi_vm::{extension_bus::ExtensionBus, extension_thread};

#[derive(Clap)]
#[clap(version = "0.1.1", author = "Muhammad Fadhlika <fadhlika@gmail.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(long)]
    path: Option<String>,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    GenerateJson,
    TestExtension(TestExtension),
}

#[derive(Clap)]
struct TestExtension {
    #[clap(long)]
    selector: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let opts: Opts = Opts::parse();

    let extension_path = match opts.path {
        Some(path) => path,
        None => "target/wasm32-wasi/release".to_string(),
    };

    let (_, extension_tx) = extension_thread::start();
    extension_thread::load(extension_path, extension_tx.clone()).await?;

    let extension_bus = ExtensionBus::new("target/wasm32-wasi/release".to_string(), extension_tx);

    match opts.subcmd {
        SubCommand::GenerateJson => {
            generate::generate_json(extension_bus).await?;
        }
        SubCommand::TestExtension(config) => {
            test::test(extension_bus, config.selector).await?;
        }
    }

    Ok(())
}
