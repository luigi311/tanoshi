use std::{collections::HashMap, error::Error, path::PathBuf};
use tanoshi_lib::prelude::Param;
use tokio::sync::mpsc::UnboundedReceiver;
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiEnv, WasiState};

use crate::catalogue::Source;

pub enum Command {
    Load(i64),
    Unload(i64),
    GetMangaList { source_id: i64, param: Param },
    GetMangaInfo { source_id: i64, path: String },
    GetChapters { source_id: i64, path: String },
    GetPages { source_id: i64, path: String },
}

pub struct Env {
    pub wasi_env: WasiEnv,
}

pub async fn extension_thread(
    extension_receiver: UnboundedReceiver<Command>,
    extension_dir_path: String,
) {
    let extension_map = HashMap::new();

    let store = Store::default();

    loop {
        let cmd = extension_receiver.recv().await;
        if let Some(cmd) = cmd {
            match cmd {
                Command::Load(source_id) => {
                    let input = Pipe::new();
                    let output = Pipe::new();
                    let mut wasi_env = WasiState::new("tanoshi")
                        .stdin(Box::new(input))
                        .stdout(Box::new(output))
                        .finalize()
                        .unwrap();

                    let extension_path = PathBuf::from(extension_dir_path);

                    let wasm_bytes = std::fs::read(extension_path).unwrap();
                    let module = Module::new(&store, wasm_bytes).unwrap();
                    let env = Env { wasi_env };

                    let import_object = wasi_env.import_object(&module).unwrap();
                    let instance = Instance::new(&module, &import_object).unwrap();

                    let detail = instance.exports.get_function("detail").unwrap();
                    detail.call(&[]).unwrap();
                    let object_str = wasi_read(&wasi_env);
                    let source: Source = ron::from_str(&object_str).unwrap();

                    print!("source: {:?}", source);
                }
                Command::Unload(source_id) => todo!(),
                Command::GetMangaList { source_id, param } => todo!(),
                Command::GetMangaInfo { source_id, path } => todo!(),
                Command::GetChapters { source_id, path } => todo!(),
                Command::GetPages { source_id, path } => todo!(),
            }
        }
    }
}

fn wasi_read(wasi_env: &WasiEnv) -> String {
    let state = wasi_env.state();
    let wasm_stdout = state.fs.stdout_mut().unwrap().as_mut().unwrap();
    let mut buf = String::new();
    wasm_stdout.read_to_string(&buf);
    buf
}

fn load(extension_path: String, config: Option<&serde_yaml::Value>) -> Result<(), Box<dyn Error>> {
    let extension_path = PathBuf::from(extension_path);

    let wasm_bytes = std::fs::read(extension_path)?;

    Ok(())
}
