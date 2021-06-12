use anyhow::{anyhow, Result};
use bytes::Bytes;
use lib::Library;
use std::path::{Path, PathBuf};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tanoshi_lib::extensions::{Extension, PluginDeclaration};
use tanoshi_lib::model::{
    Chapter, Manga, SortByParam, SortOrderParam, Source, SourceLogin, SourceLoginResult,
};

pub struct ExtensionProxy {
    extension: Arc<Box<dyn Extension>>,
    #[allow(dead_code)]
    lib: Arc<Library>,
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
        let extension = self.extension.clone();
        extension.get_mangas(keyword, genres, page, sort_by, sort_order, auth)
    }

    fn get_manga_info(&self, path: &String) -> Result<Manga> {
        let extension = self.extension.clone();
        extension.get_manga_info(path)
    }

    fn get_chapters(&self, path: &String) -> Result<Vec<Chapter>> {
        let extension = self.extension.clone();
        extension.get_chapters(path)
    }

    fn get_pages(&self, path: &String) -> Result<Vec<String>> {
        let extension = self.extension.clone();
        extension.get_pages(path)
    }

    fn get_page(&self, url: &String) -> Result<Vec<u8>> {
        let extension = self.extension.clone();
        extension.get_page(url)
    }

    fn login(&self, login_info: SourceLogin) -> Result<SourceLoginResult> {
        let extension = self.extension.clone();
        extension.login(login_info)
    }
}

pub struct Extensions {
    path: String,
    extensions: HashMap<i64, ExtensionProxy>,
}

impl Extensions {
    pub fn new(path: String) -> Extensions {
        Extensions {
            path,
            extensions: HashMap::new(),
        }
    }

    pub fn get(&self, id: i64) -> Option<&ExtensionProxy> {
        self.extensions.get(&id)
    }

    pub fn extentions(&self) -> &HashMap<i64, ExtensionProxy> {
        &self.extensions
    }

    pub fn initialize(&mut self, configs: BTreeMap<String, serde_yaml::Value>) -> Result<()> {
        match std::fs::read_dir(&self.path) {
            Ok(_) => {},
            Err(_) => {
                let _ = std::fs::create_dir_all(&self.path);
            }
        }

        for entry in std::fs::read_dir(&self.path)?
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
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_string()
                .replace("lib", "");
            info!("load plugin from {:?}", path.clone());
            unsafe {
                if let Err(e) = self.load(path.to_str().unwrap().to_string(), configs.get(&name)) {
                    error!("Error load from {:?}: {:?}", path.clone(), e);
                }
            }
        }
        Ok(())
    }

    pub unsafe fn load(
        &mut self,
        library_path: String,
        config: Option<&serde_yaml::Value>,
    ) -> Result<()> {
        let library_path = PathBuf::from(library_path);
        if cfg!(target_os = "macos") {
            if let Err(e) = std::process::Command::new("install_name_tool")
                .current_dir(library_path.parent().unwrap())
                .arg("-id")
                .arg("''")
                .arg(library_path.file_name().unwrap())
                .output()
            {
                error!("failed to run install_name_tool: {}", e);
            }
        }

        let library = Arc::new(Library::new(&library_path)?);

        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();

        if decl.rustc_version != tanoshi_lib::RUSTC_VERSION
            || decl.core_version != tanoshi_lib::CORE_VERSION
        {
            let mut error_message = "Version mismatch:".to_string();
            if decl.rustc_version != tanoshi_lib::RUSTC_VERSION {
                error_message.push_str(
                    format!(
                        " {}.rustc_version={} != tanoshi_lib.rustc_version={}",
                        decl.name,
                        decl.rustc_version,
                        tanoshi_lib::RUSTC_VERSION
                    )
                    .as_str(),
                )
            }
            if decl.core_version != tanoshi_lib::CORE_VERSION {
                error_message.push_str(
                    format!(
                        " {}.core_version={} != tanoshi_lib.core_version={}",
                        decl.name,
                        decl.core_version,
                        tanoshi_lib::CORE_VERSION
                    )
                    .as_str(),
                )
            }
            return Err(anyhow!(error_message));
        }

        let mut registrar = PluginRegistrar::new(Arc::clone(&library));
        (decl.register)(&mut registrar, config);

        self.extensions.extend(registrar.extensions);

        Ok(())
    }

    fn get_plugin_path(&self, id: i64) -> Result<PathBuf> {
        let ext = if cfg!(target_os = "windows") {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "linux") {
            "so"
        } else {
            return Err(anyhow!("os not supported"));
        };
        
        let path = Path::new(&self.path).join(format!("{}.{}", id, ext));
        Ok(path)
    }

    pub fn remove(&mut self, id: i64) -> Result<()> {
        self.extensions.remove(&id).ok_or(anyhow!("no source"))?;
        let plugin_path = self.get_plugin_path(id)?;
        std::fs::remove_file(plugin_path).map_err(|e| anyhow!("failed to remove extension: {}", e))
    }

    pub fn install(&mut self, id: i64, bytes: &Bytes) -> Result<()> {
        let plugin_path = self.get_plugin_path(id)?;
        info!("Install plugin to {:?}", plugin_path);
        std::fs::write(&plugin_path, &bytes)?;

        unsafe {
            self.load(
                plugin_path.to_str().ok_or(anyhow!("no path"))?.to_string(),
                None,
            )?;
        }

        Ok(())
    }
}

pub struct PluginRegistrar {
    extensions: HashMap<i64, ExtensionProxy>,
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
    fn register_function(&mut self, _name: &str, extension: Box<dyn Extension>) {
        let proxy = ExtensionProxy {
            extension: Arc::new(extension),
            lib: Arc::clone(&self.lib),
        };

        self.extensions.insert(proxy.detail().id, proxy);
    }
}
