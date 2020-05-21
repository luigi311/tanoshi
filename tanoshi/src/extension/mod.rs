//pub mod local;
pub mod repository;

use anyhow::{Result, anyhow};
//use local::Local;
use lib::Library;
use std::{collections::HashMap, env, ffi::OsStr, fs, io, sync::Arc};
use tanoshi_lib::extensions::{Extension, PluginDeclaration};
use tanoshi_lib::manga::{Chapter, Manga, Params, Source};

pub struct ExtensionProxy {
    extension: Box<dyn Extension>,
    _lib: Arc<Library>,
}

impl Extension for ExtensionProxy {
    fn info(&self) -> Source {
        self.extension.info()
    }
    
    fn get_mangas(&self, url: &String, param: Params, cookies: Vec<String>) -> Result<Vec<Manga>> {
        self.extension.get_mangas(url, param, cookies)
    }

    fn get_manga_info(&self, url: &String) -> Result<Manga> {
        self.extension.get_manga_info(url)
    }

    fn get_chapters(&self, url: &String) -> Result<Vec<Chapter>> {
        self.extension.get_chapters(url)
    }

    fn get_pages(&self, url: &String) -> Result<Vec<String>> {
        self.extension.get_pages(url)
    }
}

pub struct Extensions {
    extensions: HashMap<String, ExtensionProxy>,
    libraries: Vec<Arc<Library>>,
}

impl Extensions {
    pub fn new() -> Extensions {
        Extensions {
            extensions: HashMap::new(),
            libraries: vec![],
        }
    }

    pub fn extensions(&self) -> &HashMap<String, ExtensionProxy> {
       &self.extensions
    }

    pub unsafe fn load<P: AsRef<OsStr>>(&mut self, library_path: P) -> Result<()> {
        let library = Arc::new(Library::new(library_path)?);

        let decl = library.get::<*mut PluginDeclaration>(b"plugin_declaration\0")?.read();

        if decl.rustc_version != tanoshi_lib::RUSTC_VERSION || decl.core_version != tanoshi_lib::CORE_VERSION {
            return Err(anyhow!("Version mismatch"));
        }

        let mut registrar = PluginRegistrar::new(Arc::clone(&library));
        (decl.register)(&mut registrar);

        self.extensions.extend(registrar.extensions);
        self.libraries.push(library);

        Ok(())
    }
}

pub struct PluginRegistrar {
    extensions: HashMap<String, ExtensionProxy>,
    lib: Arc<Library>,
}

impl PluginRegistrar {
    fn new(lib: Arc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            lib,
            extensions: HashMap::default(),
        }
    }
}

impl tanoshi_lib::extensions::PluginRegistrar for PluginRegistrar {
    fn register_function(&mut self, name: &str, extension: Box<dyn Extension>) {
        let proxy = ExtensionProxy {
            extension,
            _lib: Arc::clone(&self.lib),
        };

        self.extensions.insert(name.to_string(), proxy);
    }
}

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
        need_login: false,
    });
    plugins
}

pub fn get_mangas(
    source: String,
    url: &String,
    param: Params,
    cookies: Vec<String>,
) -> Result<Vec<Manga>> {
    //match source.as_str() {
    //    "local" => Local::get_mangas(url, param, vec![]),
    //    &_ => {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
    let path = format!("{}/lib{}.dylib", plugin_path, source);
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<
            unsafe extern "C" fn(&String, Params, Vec<String>) -> Result<Vec<Manga>>,
        > = lib.get(b"get_mangas\0")?;
        func(url, param, cookies)
    }
    //    }
    //}
}
pub fn get_manga_info(source: String, url: &String) -> Result<Manga> {
    //match source.as_str() {
    //    "local" => Local::get_manga_info(url),
    //    &_ => {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
    let path = format!("{}/lib{}.dylib", plugin_path, source);
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<unsafe extern "C" fn(&String) -> Result<Manga>> =
            lib.get(b"get_manga_info\0")?;
        func(url)
    }
    //    }
    //}
}
pub fn get_chapters(source: String, url: &String) -> Result<Vec<Chapter>> {
    //match source.as_str() {
    //    "local" => Local::get_chapters(url),
    //    &_ => {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
    let path = format!("{}/lib{}.dylib", plugin_path, source);
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<unsafe extern "C" fn(&String) -> Result<Vec<Chapter>>> =
            lib.get(b"get_chapters\0")?;
        func(url)
    }
    //    }
    //}
}
pub fn get_pages(source: String, url: &String) -> Result<Vec<String>> {
    //match source.as_str() {
    //    "local" => Local::get_pages(url),
    //    &_ => {
    let plugin_path = env::var("PLUGIN_PATH").expect("PLUGIN_PATH not set");
    let path = format!("{}/lib{}.dylib", plugin_path, source);
    let lib = lib::Library::new(path)?;
    unsafe {
        let func: lib::Symbol<unsafe extern "C" fn(&String) -> Result<Vec<String>>> =
            lib.get(b"get_pages\0")?;
        func(url)
    }
    //    }
    //}
}
