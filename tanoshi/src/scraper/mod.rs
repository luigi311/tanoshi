pub mod local;
pub mod repository;

use anyhow::Result;
use local::Local;
use std::{env, fs};
use tanoshi::manga::{Chapter, Manga, Params, Source};
use tanoshi::scraping::Scraping;

fn get_source(path: &str) -> Result<Source> {
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<unsafe extern "C" fn() -> Source> =
            lib.get(b"get_source_detail\0")?;
        Ok(func())
    }
}

pub fn get_sources() -> Vec<Source> {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");

    let mut plugins = fs::read_dir(plugin_path)
        .expect("error read directory")
        .filter_map(
            |res| match res.map(|e| get_source(e.path().to_str().unwrap_or(""))) {
                Ok(n) => n.ok(),
                Err(e) => None,
            },
        )
        .collect::<Vec<_>>();
    let local_path = std::env::var("MANGA_PATH").expect("MANGA_PATH not set");
    plugins.push(Source {
        id: 0,
        name: "local".to_string(),
        url: local_path,
    });
    plugins
}

pub fn get_mangas(
    source: String,
    url: &String,
    param: Params,
    cookies: Vec<String>,
) -> Result<Vec<Manga>> {
    match source.as_str() {
        "local" => Local::get_mangas(url, param, vec![]),
        &_ => {
            let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
            let path = format!("{}/lib{}.dylib", plugin_path, source);
            let lib = lib::Library::new(path)?;
            unsafe {
                let func: lib::Symbol<
                    unsafe extern "C" fn(&String, Params, Vec<String>) -> Result<Vec<Manga>>,
                > = lib.get(b"get_mangas\0")?;
                func(url, param, cookies)
            }
        }
    }
}
pub fn get_manga_info(source: String, url: &String) -> Result<Manga> {
    match source.as_str() {
        "local" => Local::get_manga_info(url),
        &_ => {
            let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
            let path = format!("{}/lib{}.dylib", plugin_path, source);
            let lib = lib::Library::new(path)?;
            unsafe {
                let func: lib::Symbol<unsafe extern "C" fn(&String) -> Result<Manga>> =
                    lib.get(b"get_manga_info\0")?;
                func(url)
            }
        }
    }
}
pub fn get_chapters(source: String, url: &String) -> Result<Vec<Chapter>> {
    match source.as_str() {
        "local" => Local::get_chapters(url),
        &_ => {
            let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
            let path = format!("{}/lib{}.dylib", plugin_path, source);
            let lib = lib::Library::new(path)?;
            unsafe {
                let func: lib::Symbol<unsafe extern "C" fn(&String) -> Result<Vec<Chapter>>> =
                    lib.get(b"get_chapters\0")?;
                func(url)
            }
        }
    }
}
pub fn get_pages(source: String, url: &String) -> Result<Vec<String>> {
    match source.as_str() {
        "local" => Local::get_pages(url),
        &_ => {
            let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
            let path = format!("{}/lib{}.dylib", plugin_path, source);
            let lib = lib::Library::new(path)?;
            unsafe {
                let func: lib::Symbol<unsafe extern "C" fn(&String) -> Result<Vec<String>>> =
                    lib.get(b"get_pages\0")?;
                func(url)
            }
        }
    }
}
