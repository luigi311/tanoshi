extern crate libloading as lib;

mod extension;

use anyhow::Result;

#[derive(serde::Serialize)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub rustc_version: String,
    pub core_version: String,
    pub version: String,
}

fn main() -> Result<()> {
    let repo = format!("repo-{}", std::env::consts::OS);
    let path = std::path::Path::new(&repo);
    let _ = std::fs::create_dir_all(path.join("library"));

    let mut sources = vec![];
    let mut exts = extension::Extensions::new();
    for entry in std::fs::read_dir("target/release")?
        .into_iter()
        .filter(move |path| {
            if let Ok(p) = path {
                let ext = p
                    .clone()
                    .path()
                    .extension()
                    .unwrap_or("".as_ref())
                    .to_owned();
                if ext == "so" || ext == "dll" || ext == "dylib" {
                    return true;
                }
            }
            return false;
        })
    {
        let path = entry?.path();
        unsafe {
            let source = exts.load(path.to_str().unwrap().to_string(), None)?;
            sources.push(source);
        }
    }

    let file = std::fs::File::create(path.join("index.json"))?;
    serde_json::to_writer(&file, &sources)?;
    Ok(())
}
