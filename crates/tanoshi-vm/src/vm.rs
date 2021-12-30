use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use pathdiff::diff_paths;
use rquickjs::{
    BuiltinLoader, BuiltinResolver, FileResolver, ModuleLoader, NativeLoader, Runtime,
    ScriptLoader, Tokio,
};

#[cfg(target_os = "linux")]
extern "C" {
    fn malloc_trim(pad: usize) -> std::os::raw::c_int;
}

pub fn create_runtime<P: AsRef<Path>>(extension_dir: P) -> Result<Runtime> {
    let extension_dir =
        if let Some(relative_path) = diff_paths(&extension_dir, env::current_dir().unwrap()) {
            relative_path
        } else {
            PathBuf::new().join(extension_dir)
        };

    let rt = Runtime::new()?;

    let resolver = (
        BuiltinResolver::default(),
        FileResolver::default()
            .with_path(
                extension_dir
                    .to_str()
                    .ok_or(anyhow!("failed to convert path to_str"))?,
            )
            .with_pattern("{}.mjs")
            .with_native(),
    );

    let loader = (
        BuiltinLoader::default(),
        ModuleLoader::default(),
        ScriptLoader::default().with_extension("mjs"),
        NativeLoader::default(),
    );

    rt.set_loader(resolver, loader);
    rt.spawn_executor(Tokio);

    #[cfg(target_os = "linux")]
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(6 * 60 * 60)).await;
            unsafe { malloc_trim(0) };
        }
    });

    Ok(rt)
}
