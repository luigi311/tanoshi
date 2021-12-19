use std::collections::HashMap;

use anyhow::Result;
use rquickjs::{Context, Function, Persistent, Promise};
use tanoshi_vm::{
    api::{BytesToString, Console, Fetch, Print},
    vm::create_runtime,
};
use tokio::io::AsyncReadExt;

pub async fn test(filename: Option<String>) -> Result<()> {
    let rt = create_runtime(".")?;
    let ctx = Context::full(&rt)?;

    let source = if let Some(filename) = filename {
        tokio::fs::read_to_string(filename)
            .await?
            .as_bytes()
            .to_vec()
    } else {
        let mut buf = vec![];
        let mut stdin = tokio::io::stdin();
        stdin.read_to_end(&mut buf).await?;
        buf
    };

    let mut tests = HashMap::new();

    ctx.with(|ctx| -> Result<_> {
        let global = ctx.globals();
        global.init_def::<Print>()?;
        global.init_def::<Console>()?;
        global.init_def::<Fetch>()?;
        global.init_def::<BytesToString>()?;

        let module = ctx.compile("run", source)?;

        for entry in module.entries::<String, Function>() {
            if let Ok((name, test)) = entry {
                tests.insert(name, Persistent::save(ctx, test));
            }
        }

        Ok(())
    })?;

    for (_, test) in tests {
        let func: Promise<()> = ctx.with(|ctx| test.restore(ctx)?.call(()))?;
        func.await?;
    }

    Ok(())
}
