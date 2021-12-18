use anyhow::Result;
use rquickjs::{Context, Function, Promise};
use tanoshi_vm::{
    api::{BytesToString, Console, Fetch, Print},
    vm::create_runtime,
};

pub async fn test(filename: &str) -> Result<()> {
    let rt = create_runtime(".")?;
    let ctx = Context::full(&rt)?;

    let source = tokio::fs::read_to_string(filename).await?;

    let main: Promise<()> = ctx.with(|ctx| -> Result<_> {
        let global = ctx.globals();
        global.init_def::<Print>()?;
        global.init_def::<Console>()?;
        global.init_def::<Fetch>()?;
        global.init_def::<BytesToString>()?;

        let module = ctx.compile("run", source)?;

        Ok(module.get::<_, Function>("main")?.call(())?)
    })?;

    main.await?;

    Ok(())
}
