use std::collections::HashMap;

use anyhow::{anyhow, Result};
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

        for (name, test) in module.entries::<String, Function>().flatten() {
            tests.insert(name, Persistent::save(ctx, test));
        }

        Ok(())
    })?;

    let test_len = tests.len();
    eprintln!("running {} test", test_len);

    let mut failures = HashMap::new();
    for (name, test) in tests {
        eprint!("test {} ... ", name);
        let func: Promise<()> = ctx.with(|ctx| test.restore(ctx)?.call(()))?;
        if let Err(e) = func.await {
            eprint!("FAILED");
            failures.insert(name.clone(), e.to_string());
        } else {
            eprint!("ok");
        }
        eprintln!();
    }

    let failures_len = failures.len();

    if failures_len > 0 {
        eprintln!("---failures---");
    }
    for (name, failure) in failures {
        eprintln!("{}", name);
        eprintln!("\t{}", failure);
    }

    eprintln!(
        "test result: {}. {} passed; {} failed",
        if failures_len == 0 { "ok" } else { "FAILED" },
        test_len - failures_len,
        failures_len
    );

    if failures_len > 0 {
        return Err(anyhow!("{}", failures_len));
    }

    Ok(())
}
