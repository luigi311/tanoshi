extern crate log;

mod data;
mod generate;
mod test;

use clap::Parser;
use tanoshi_vm::vm;

#[derive(Parser)]
#[clap(version = "0.1.1", author = "Muhammad Fadhlika <fadhlika@gmail.com>")]
struct Opts {
    #[clap(short, long, default_value = "target/wasm32-wasi/release")]
    path: String,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    #[cfg(not(feature = "disable-compiler"))]
    Compile(CompileOption),
    GenerateJson,
    Test(TestOption),
}

#[derive(Parser)]
struct TestOption {
    #[clap(long)]
    selector: Option<String>,
}

#[derive(Parser)]
struct CompileOption {
    #[clap(long)]
    target: String,
    #[clap(long)]
    remove_wasm: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let opts: Opts = Opts::parse();

    let (_, extension_bus) = vm::start(&opts.path);

    #[cfg(not(feature = "disable-compiler"))]
    if !matches!(opts.subcmd, SubCommand::Compile(_)) {
        extension_bus.load()?;
    }

    #[cfg(feature = "disable-compiler")]
    vm::load(&opts.path, extension_tx.clone())?;

    match opts.subcmd {
        #[cfg(not(feature = "disable-compiler"))]
        SubCommand::Compile(compile_opts) => {
            // let triples = [
            //     "x86_64-apple-darwin",
            //     "x86_64-pc-windows-msvc",
            //     "x86_64-unknown-linux-gnu",
            //     "aarch64-unknown-linux-gnu",
            // ];
            vm::compile_with_target(&opts.path, &compile_opts.target, compile_opts.remove_wasm)?;
        }
        SubCommand::GenerateJson => {
            generate::generate_json(extension_bus).await?;
        }
        SubCommand::Test(config) => {
            test::test(extension_bus, config.selector).await?;
        }
    }

    Ok(())
}
