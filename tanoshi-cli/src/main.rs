#[macro_use]
extern crate log;

mod extension;

use anyhow::Result;
use tanoshi_lib::prelude::Extension;

#[derive(serde::Serialize)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub version: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let path = std::path::Path::new("repo");
    let _ = std::fs::create_dir_all(path.join("library"));
    
    let extensions = extension::load("target/wasm32-wasi/release".to_string());

    let sources = extensions.iter().map(|(_, ext)| {
        let detail = ext.detail();
        Source {
            id: detail.id,
            name: detail.name.clone(),
            path: format!("library/{}.wasm", detail.name),
            version: detail.version,
        }
    }).collect::<Vec<Source>>();

    let file = std::fs::File::create(path.join("index.json"))?;
    serde_json::to_writer(&file, &sources)?;
    Ok(())
}
