use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use fnv::FnvHashMap;
use libloading::Library;
use tanoshi_lib::prelude::{Input, Lang, PluginDeclaration, SourceInfo};

use crate::{
    PLUGIN_EXTENSION,
    prelude::{Source, SourceEntry},
};

#[derive(Clone)]
pub struct ExtensionManager {
    dir: PathBuf,
    extensions: Arc<RwLock<FnvHashMap<i64, Arc<SourceEntry>>>>,
}

pub fn dummy_source_info(id: i64) -> SourceInfo {
    SourceInfo {
        id,
        name: format!("Missing source {id}"),
        url: String::new(),
        version: "",
        icon: "",
        languages: Lang::All,
        nsfw: false,
    }
}

impl ExtensionManager {
    pub fn new<P: AsRef<Path>>(extension_dir: P) -> Self {
        Self {
            dir: PathBuf::new().join(extension_dir),
            extensions: Arc::new(RwLock::new(FnvHashMap::default())),
        }
    }

    fn read(&self) -> Result<RwLockReadGuard<'_, FnvHashMap<i64, Arc<SourceEntry>>>> {
        self.extensions
            .read()
            .map_err(|e| anyhow!("failed to lock read: {e}"))
    }

    fn write(&self) -> Result<RwLockWriteGuard<'_, FnvHashMap<i64, Arc<SourceEntry>>>> {
        self.extensions
            .write()
            .map_err(|e| anyhow!("failed to lock write: {e}"))
    }

    fn entry(&self, source_id: i64) -> Result<Arc<SourceEntry>> {
        self.read()?
            .get(&source_id)
            .cloned()
            .ok_or_else(|| anyhow!("no such source"))
    }

    async fn call_blocking<T, F>(&self, source_id: i64, call: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
    {
        let entry = self.entry(source_id)?;
        tokio::task::spawn_blocking(move || entry.with_extension(call)).await?
    }

    pub async fn load_all(&self) -> Result<()> {
        let mut read_dir = tokio::fs::read_dir(&self.dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(PLUGIN_EXTENSION)
                && let Err(e) = self.load(&name).await
            {
                error!("failed to load {name}: {e}");
            }
        }
        Ok(())
    }

    pub async fn exists(&self, source_id: i64) -> Result<bool> {
        Ok(self.read()?.get(&source_id).is_some())
    }

    pub async fn list(&self) -> Result<Vec<SourceInfo>> {
        let entries = self.read()?.values().cloned().collect::<Vec<_>>();
        entries
            .into_iter()
            .map(|entry| entry.source_info())
            .collect()
    }

    pub async fn install(&self, repo_url: &str, name: &str) -> Result<()> {
        let source_file_url = format!(
            "{}/{}/{}.{}",
            repo_url,
            env!("TARGET"),
            name.to_lowercase(),
            PLUGIN_EXTENSION
        );

        info!("downloading {source_file_url}");

        let contents = reqwest::get(&source_file_url).await?.bytes().await?;

        tokio::fs::write(
            self.dir
                .join(name.to_lowercase())
                .with_extension(PLUGIN_EXTENSION),
            contents,
        )
        .await?;

        let source = self.load_library(&name.to_lowercase())?;
        self.insert(source).await?;

        info!("installed extension {name}");

        Ok(())
    }

    fn load_library(&self, name: &str) -> Result<Source> {
        let library_path = PathBuf::new()
            .join(&self.dir)
            .join(name)
            .with_extension(PLUGIN_EXTENSION);
        info!("load {:?}", library_path.display());

        #[cfg(target_os = "macos")]
        if let Err(e) = std::process::Command::new("install_name_tool")
            .current_dir(library_path.parent().unwrap())
            .arg("-id")
            .arg("''")
            .arg(library_path.file_name().unwrap())
            .output()
        {
            error!("failed to run install_name_tool: {e}");
        }

        unsafe {
            let library = Library::new(&library_path)?;

            let decl = library
                .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
                .read();

            if decl.rustc_version != tanoshi_lib::RUSTC_VERSION {
                bail!(
                    "Version mismatch: extension.rustc_version={} != tanoshi_lib.rustc_version={}",
                    decl.rustc_version,
                    tanoshi_lib::RUSTC_VERSION,
                );
            }

            if decl.core_version != tanoshi_lib::LIB_VERSION {
                bail!(
                    "Version mismatch: extension.lib_version={} != tanoshi_lib::lib_version={}",
                    decl.core_version,
                    tanoshi_lib::LIB_VERSION
                );
            }

            let mut registrar = Source::new(library, decl.rustc_version, decl.core_version);
            (decl.register)(&mut registrar);

            Ok(registrar)
        }
    }

    pub async fn load(&self, name: &str) -> Result<()> {
        let mut source = self.load_library(name)?;
        let source_name = source
            .extension
            .get()
            .ok_or_else(|| anyhow!("not initiated"))?
            .get_source_info()
            .name
            .to_lowercase();

        if let Some(preferences) =
            tokio::fs::read_to_string(self.dir.join(&source_name).with_extension("json"))
                .await
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        {
            info!("set preferences for {}", source_name);
            source
                .extension
                .get_mut()
                .ok_or_else(|| anyhow!("not initiated"))?
                .set_preferences(preferences)?;
        }
        self.insert(source).await
    }

    pub async fn insert(&self, source: Source) -> Result<()> {
        let entry = Arc::new(source.into_entry()?);
        self.write()?.insert(entry.source_id, entry);
        Ok(())
    }

    pub async fn unload(&self, source_id: i64) -> Result<()> {
        if let Some(entry) = self.write()?.remove(&source_id) {
            std::fs::remove_file(
                self.dir
                    .join(entry.source_name().to_lowercase())
                    .with_extension(PLUGIN_EXTENSION),
            )?;

            info!(
                "uninstalled extension {source_id} ({})",
                entry.source_name()
            );
        }
        Ok(())
    }

    pub async fn remove(&self, source_id: i64) -> Result<()> {
        self.unload(source_id).await
    }

    pub fn get_version(&self, source_id: i64) -> Result<(String, String)> {
        let sources = self.read()?;
        let source = sources
            .get(&source_id)
            .ok_or_else(|| anyhow!("no such source"))?;
        let rustc_version = source.rustc_version.clone();
        let lib_version = source.lib_version.clone();
        Ok((rustc_version, lib_version))
    }

    pub fn get_source_info(&self, source_id: i64) -> Result<SourceInfo> {
        let entry = self.read()?.get(&source_id).cloned();
        if let Some(entry) = entry {
            entry.source_info()
        } else {
            println!("Returning dummy source info");
            Ok(dummy_source_info(source_id))
        }
    }

    pub fn filter_list(&self, source_id: i64) -> Result<Vec<Input>> {
        let entry = self.entry(source_id)?;
        entry.with_extension(|extension| Ok(extension.filter_list()))
    }

    pub fn get_preferences(&self, source_id: i64) -> Result<Vec<Input>> {
        let entry = self.entry(source_id)?;
        entry.with_extension(|extension| extension.get_preferences())
    }

    pub async fn set_preferences(&self, source_id: i64, preferences: Vec<Input>) -> Result<()> {
        let entry = self.entry(source_id)?;
        entry.with_extension_mut(|extension| extension.set_preferences(preferences.clone()))?;

        let source_info = entry.source_info()?;
        tokio::fs::write(
            self.dir
                .join(source_info.name.to_lowercase())
                .with_extension("json"),
            serde_json::to_string_pretty(&preferences)?,
        )
        .await?;

        Ok(())
    }

    pub async fn get_popular_manga(
        &self,
        source_id: i64,
        page: i64,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        self.call_blocking(source_id, move |extension| {
            extension.get_popular_manga(page)
        })
        .await
    }

    pub async fn get_latest_manga(
        &self,
        source_id: i64,
        page: i64,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        self.call_blocking(source_id, move |extension| extension.get_latest_manga(page))
            .await
    }

    pub async fn search_manga(
        &self,
        source_id: i64,
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        self.call_blocking(source_id, move |extension| {
            extension.search_manga(page, query, filters)
        })
        .await
    }

    pub async fn get_manga_detail(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<tanoshi_lib::prelude::MangaInfo> {
        self.call_blocking(source_id, move |extension| extension.get_manga_detail(path))
            .await
    }

    pub async fn get_chapters(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<tanoshi_lib::prelude::ChapterInfo>> {
        self.call_blocking(source_id, move |extension| extension.get_chapters(path))
            .await
    }

    pub async fn get_pages(&self, source_id: i64, path: String) -> Result<Vec<String>> {
        self.call_blocking(source_id, move |extension| extension.get_pages(path))
            .await
    }

    pub async fn get_image_bytes(&self, source_id: i64, url: String) -> Result<Bytes> {
        self.call_blocking(source_id, move |extension| extension.get_image_bytes(url))
            .await
    }
}
