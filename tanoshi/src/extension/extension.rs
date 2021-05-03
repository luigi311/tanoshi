use lib::Library;
use std::path::PathBuf;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tanoshi_lib::extensions::{Extension, PluginDeclaration};
use tanoshi_lib::manga::{Chapter, Manga, Params, Source, SourceLogin, SourceLoginResult};

use anyhow::{anyhow, Result};

pub struct ExtensionProxy {
    extension: Box<dyn Extension>,
    #[allow(dead_code)]
    lib: Arc<Library>,
}

impl Extension for ExtensionProxy {
    fn info(&self) -> Source {
        self.extension.info()
    }

    fn get_mangas(&self, param: Params, auth: String) -> Result<Vec<Manga>> {
        self.extension.get_mangas(param, auth)
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

    fn get_page(&self, url: &String) -> Result<Vec<u8>> {
        self.extension.get_page(url)
    }

    fn login(&self, login_info: SourceLogin) -> Result<SourceLoginResult> {
        self.extension.login(login_info)
    }
}

pub struct Extensions {
    extensions: HashMap<String, ExtensionProxy>,
}

impl Extensions {
    pub fn new() -> Extensions {
        Extensions {
            extensions: HashMap::new(),
        }
    }

    pub fn initialize<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
        config: BTreeMap<String, serde_yaml::Value>,
    ) -> Result<()> {
        for entry in std::fs::read_dir(path.as_ref())?
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
                self.load(path.to_str().unwrap().to_string(), config.get(&name))
                    .unwrap()
            }
        }
        Ok(())
    }

    pub fn get(&self, name: &String) -> Option<&ExtensionProxy> {
        self.extensions.get(name)
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
            return Err(anyhow!("Version mismatch: extension.rustc_version={}, extension.core_version={}, tanoshi_lib.rustc_version={}, tanoshi_lib::core_version={}", 
                decl.rustc_version , decl.core_version, tanoshi_lib::RUSTC_VERSION, tanoshi_lib::CORE_VERSION)
            );
        }

        let mut registrar = PluginRegistrar::new(Arc::clone(&library));
        (decl.register)(&mut registrar, config);

        self.extensions.extend(registrar.extensions);

        Ok(())
    }

    pub fn remove(&mut self, name: &String) -> Result<()> {
        if self.extensions.remove(name).is_some() {
            Ok(())
        } else {
            Err(anyhow!("There is no extension {}", &name))
        }
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
            lib: Arc::clone(&self.lib),
        };

        self.extensions.insert(name.to_string(), proxy);
    }
}
