pub mod repository;
pub mod local;

use anyhow::Result;
use std::env;
use tanoshi::manga::{
    Manga, Chapter, Params
};

pub fn get_mangas(source: String, url: &String, param: Params, cookies: Vec<String>) -> Result<Vec<Manga>>  {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
    let path = format!("{}/lib{}.dylib", plugin_path, source);
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<unsafe extern fn(&String, Params, Vec<String>) -> Result<Vec<Manga>>> = lib.get(b"get_mangas\0")?;
        func(url, param, cookies)
    }
}
pub fn get_manga_info(source: String, url: &String) -> Result<Manga> {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
    let path = format!("{}/lib{}.dylib", plugin_path, source);
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<unsafe extern fn(&String) -> Result<Manga>>  = lib.get(b"get_manga_info\0")?;
        func(url)
    }
}
pub fn get_chapters(source: String, url: &String) -> Result<Vec<Chapter>> {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
    let path = format!("{}/lib{}.dylib", plugin_path, source);
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<unsafe extern fn(&String) -> Result<Vec<Chapter>>> = lib.get(b"get_chapters\0")?;
        func(url)
    }
}
pub fn get_pages(source: String, url: &String) -> Result<Vec<String>> {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
    let path = format!("{}/lib{}.dylib", plugin_path, source);
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<unsafe extern fn(&String) -> Result<Vec<String>>> = lib.get(b"get_pages\0")?;
        func(url)
    }
}