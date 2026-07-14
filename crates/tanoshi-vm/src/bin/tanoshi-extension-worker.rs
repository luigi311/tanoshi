use std::path::PathBuf;

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args_os();
    let _program = args.next();
    let Some(mut flag) = args.next() else {
        bail!("usage: tanoshi-extension-worker --plugin <path>");
    };
    if flag == "--tanoshi-extension-worker" {
        flag = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("usage: tanoshi-extension-worker --plugin <path>"))?;
    }
    if flag != "--plugin" {
        bail!("usage: tanoshi-extension-worker --plugin <path>");
    }
    let Some(plugin_path) = args.next() else {
        bail!("usage: tanoshi-extension-worker --plugin <path>");
    };
    if args.next().is_some() {
        bail!("usage: tanoshi-extension-worker --plugin <path>");
    }

    tanoshi_vm::extension::worker::run_worker(PathBuf::from(plugin_path))
}
