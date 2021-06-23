use anyhow::{anyhow, Result};
use lib::Library;
use tanoshi_lib::model::{SortByParam, SortOrderParam};
use std::{collections::HashMap, sync::Arc};
use tanoshi_lib::extensions::{Extension, PluginDeclaration};
use tanoshi_lib::model::{Chapter, Manga, Source, SourceLogin, SourceLoginResult};

pub struct ExtensionProxy {
    extension: Box<dyn Extension>,
    _lib: Arc<Library>,
}

impl Extension for ExtensionProxy {
    fn detail(&self) -> Source {
        self.extension.detail()
    }

    fn get_mangas(
        &self,
        keyword: Option<String>,
        genres: Option<Vec<String>>,
        page: Option<i32>,
        sort_by: Option<SortByParam>,
        sort_order: Option<SortOrderParam>,
        auth: Option<String>,
    ) -> Result<Vec<Manga>> {
        self.extension.get_mangas(keyword, genres, page, sort_by, sort_order, auth)
    }

    fn get_manga_info(&self, path: &String) -> Result<Manga> {
        self.extension.get_manga_info(path)
    }

    fn get_chapters(&self, path: &String) -> Result<Vec<Chapter>> {
        self.extension.get_chapters(path)
    }

    fn get_pages(&self, path: &String) -> Result<Vec<String>> {
        self.extension.get_pages(path)
    }

    fn get_page(&self, path: &String) -> Result<Vec<u8>> {
        self.extension.get_page(path)
    }

    fn login(&self, login_info: SourceLogin) -> Result<SourceLoginResult> {
        self.extension.login(login_info)
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

    /* pub fn get(&self, name: &String) -> Option<&ExtensionProxy> {
        self.extensions.get(name)
    } */

    pub unsafe fn load(
        &mut self,
        library_path: String,
        config: Option<&serde_yaml::Value>,
    ) -> Result<crate::Source> {
        let library = Arc::new(Library::new(&library_path)?);

        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();

        if decl.rustc_version != tanoshi_lib::RUSTC_VERSION
            || decl.core_version != tanoshi_lib::CORE_VERSION
        {
            return Err(anyhow!("Version mismatch"));
        }

        let ext = if cfg!(target_os = "windows") {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "linux") {
            "so"
        } else {
            return Err(anyhow!("os not supported"));
        };

        let mut registrar = PluginRegistrar::new(Arc::clone(&library));
        (decl.register)(&mut registrar, config);

        self.extensions.extend(registrar.extensions);
        self.libraries.push(library);

        let detail = self.extensions().get(decl.name).unwrap().detail();
        
        let new_filename = format!("{}-v{}.{}", &decl.name, detail.version, ext);
        let mut new_path = format!("repo-{}/library/{}", std::env::consts::OS, new_filename);
        if cfg!(target_os = "windows") {
            new_path = new_path.replace("/", "\\");
        }
        let _ = std::fs::copy(library_path, &new_path);

        let path = format!("library/{}", &new_filename);
        Ok(crate::Source {
            id: detail.id,
            name: decl.name.to_string(),
            path: path.to_string(),
            rustc_version: decl.rustc_version.to_string(),
            core_version: decl.core_version.to_string(),
            version: detail.version,
        })
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
