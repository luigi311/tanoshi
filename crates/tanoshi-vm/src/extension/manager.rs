use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex as StdMutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use fnv::FnvHashMap;
use libloading::Library;
use tanoshi_lib::prelude::{Input, Lang, PluginDeclaration, SourceInfo};
use tokio::sync::{Mutex as AsyncMutex, OwnedSemaphorePermit};

use crate::{
    PLUGIN_EXTENSION,
    prelude::{Source, SourceEntry},
};

const STAGED_LIBRARY_PREFIX: &str = ".tanoshi-staged-";
const INSTALL_TEMP_PREFIX: &str = ".tanoshi-install-";
const INSTALL_BACKUP_PREFIX: &str = ".tanoshi-backup-";
static UNIQUE_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);
const DEFAULT_MAX_CONCURRENT_CALLS: usize = 8;
const DEFAULT_ADMISSION_TIMEOUT: Duration = Duration::from_secs(1);

fn is_managed_library_name(name: &str) -> bool {
    name.starts_with(STAGED_LIBRARY_PREFIX)
        || name.starts_with(INSTALL_TEMP_PREFIX)
        || name.starts_with(INSTALL_BACKUP_PREFIX)
}

fn normalize_plugin_name(name: &str) -> String {
    let name = Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(name);
    let extension = format!(".{PLUGIN_EXTENSION}");
    name.strip_suffix(&extension).unwrap_or(name).to_lowercase()
}

fn entry_plugin_name(entry: &SourceEntry) -> String {
    entry
        .plugin_path()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(normalize_plugin_name)
        .unwrap_or_else(|| normalize_plugin_name(entry.source_name()))
}

fn cleanup_managed_libraries(dir: &Path) {
    let current_process_marker = format!("-{}-", std::process::id());
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            warn!(
                "failed to scan {} for stale extension files: {error}",
                dir.display()
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !is_managed_library_name(name) {
            continue;
        }
        if name.contains(&current_process_marker) {
            continue;
        }

        if let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                "failed to remove stale extension file {}: {error}",
                path.display()
            );
        }
    }
}

struct PluginFileReplacement {
    backup_path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
pub struct ExtensionManagerOptions {
    pub max_concurrent_calls: usize,
    pub admission_timeout: Duration,
}

impl Default for ExtensionManagerOptions {
    fn default() -> Self {
        Self {
            max_concurrent_calls: DEFAULT_MAX_CONCURRENT_CALLS,
            admission_timeout: DEFAULT_ADMISSION_TIMEOUT,
        }
    }
}

#[derive(Clone)]
pub struct ExtensionManager {
    dir: PathBuf,
    extensions: Arc<RwLock<FnvHashMap<i64, Arc<SourceEntry>>>>,
    lifecycle_locks: Arc<StdMutex<FnvHashMap<String, Arc<AsyncMutex<()>>>>>,
    options: ExtensionManagerOptions,
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
        Self::new_with_options(extension_dir, ExtensionManagerOptions::default())
    }

    pub fn new_with_options<P: AsRef<Path>>(
        extension_dir: P,
        mut options: ExtensionManagerOptions,
    ) -> Self {
        if options.max_concurrent_calls == 0 {
            warn!("configured extension concurrency is zero; using one call per source instead");
            options.max_concurrent_calls = 1;
        }
        let dir = PathBuf::new().join(extension_dir);
        cleanup_managed_libraries(&dir);
        Self {
            dir,
            extensions: Arc::new(RwLock::new(FnvHashMap::default())),
            lifecycle_locks: Arc::new(StdMutex::new(FnvHashMap::default())),
            options,
        }
    }

    fn lifecycle_lock(&self, name: &str) -> Result<Arc<AsyncMutex<()>>> {
        let mut locks = self
            .lifecycle_locks
            .lock()
            .map_err(|error| anyhow!("failed to lock plugin lifecycle map: {error}"))?;
        Ok(locks
            .entry(normalize_plugin_name(name))
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone())
    }

    fn unique_managed_path(&self, source_path: &Path, prefix: &str) -> Result<PathBuf> {
        let file_name = source_path
            .file_name()
            .ok_or_else(|| anyhow!("extension path has no file name"))?
            .to_string_lossy();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let counter = UNIQUE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "{prefix}{}-{timestamp}-{counter}-{file_name}",
            std::process::id()
        );
        Ok(source_path.parent().unwrap_or(&self.dir).join(name))
    }

    fn stage_library(&self, plugin_path: &Path) -> Result<PathBuf> {
        let staged_path = self.unique_managed_path(plugin_path, STAGED_LIBRARY_PREFIX)?;
        if let Err(error) = std::fs::copy(plugin_path, &staged_path) {
            let _ = std::fs::remove_file(&staged_path);
            return Err(error.into());
        }
        Ok(staged_path)
    }

    fn cleanup_managed_file(path: &Path) {
        if let Err(error) = std::fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                "failed to remove managed extension file {}: {error}",
                path.display()
            );
        }
    }

    async fn replace_plugin_file(
        &self,
        temporary_path: &Path,
        plugin_path: &Path,
    ) -> Result<PluginFileReplacement> {
        let backup_path = match tokio::fs::metadata(plugin_path).await {
            Ok(_) => {
                let backup_path = self.unique_managed_path(plugin_path, INSTALL_BACKUP_PREFIX)?;
                tokio::fs::rename(plugin_path, &backup_path).await?;
                Some(backup_path)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };

        if let Err(error) = tokio::fs::rename(temporary_path, plugin_path).await {
            if let Some(backup_path) = backup_path.as_ref()
                && let Err(restore_error) = tokio::fs::rename(backup_path, plugin_path).await
            {
                return Err(anyhow!(
                    "failed to replace extension {}: {error}; failed to restore the previous file: {restore_error}",
                    plugin_path.display()
                ));
            }
            return Err(error.into());
        }

        Ok(PluginFileReplacement { backup_path })
    }

    async fn rollback_plugin_file(
        &self,
        plugin_path: &Path,
        replacement: PluginFileReplacement,
    ) -> Result<()> {
        match tokio::fs::remove_file(plugin_path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        if let Some(backup_path) = replacement.backup_path {
            tokio::fs::rename(backup_path, plugin_path).await?;
        }
        Ok(())
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

    async fn acquire_permit(
        &self,
        entry: &Arc<SourceEntry>,
        operation: &'static str,
    ) -> Result<OwnedSemaphorePermit> {
        let source_id = entry.source_id;
        let source_name = entry.source_name().to_owned();
        match tokio::time::timeout(
            self.options.admission_timeout,
            entry.limiter.clone().acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(error)) => {
                error!(
                    "EXTENSION ADMISSION ERROR: source_id={source_id} source={source_name:?} operation={operation} limiter unavailable: {error}"
                );
                bail!(
                    "[extension-admission] source {source_id} ({source_name}) cannot accept {operation}: {error}"
                );
            }
            Err(_) => {
                error!(
                    "EXTENSION SATURATION: source_id={source_id} source={source_name:?} operation={operation} exceeded the {}-call limit; request rejected after {:?}",
                    self.options.max_concurrent_calls, self.options.admission_timeout
                );
                bail!(
                    "[extension-saturated] source {source_id} ({source_name}) is busy; {operation} admission timed out after {:?}",
                    self.options.admission_timeout
                );
            }
        }
    }

    async fn call_blocking<T, F>(
        &self,
        source_id: i64,
        operation: &'static str,
        call: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
    {
        let entry = self.entry(source_id)?;
        self.call_blocking_entry(entry, operation, call).await
    }

    async fn call_blocking_entry<T, F>(
        &self,
        entry: Arc<SourceEntry>,
        operation: &'static str,
        call: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
    {
        let permit = self.acquire_permit(&entry, operation).await?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            entry.with_extension(call)
        })
        .await?
    }

    async fn call_blocking_mut_entry<T, F>(
        &self,
        entry: Arc<SourceEntry>,
        operation: &'static str,
        call: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
    {
        let permit = self.acquire_permit(&entry, operation).await?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            entry.with_extension_mut(call)
        })
        .await?
    }

    pub async fn load_all(&self) -> Result<()> {
        let mut read_dir = tokio::fs::read_dir(&self.dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(PLUGIN_EXTENSION)
                && !is_managed_library_name(&name)
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
        Ok(entries
            .into_iter()
            .map(|entry| entry.source_info.clone())
            .collect())
    }

    pub async fn install(&self, repo_url: &str, name: &str) -> Result<()> {
        let plugin_name = normalize_plugin_name(name);
        let source_file_url = format!(
            "{}/{}/{}.{}",
            repo_url,
            env!("TARGET"),
            plugin_name,
            PLUGIN_EXTENSION
        );

        info!("downloading {source_file_url}");

        let contents = reqwest::get(&source_file_url).await?.bytes().await?;
        let plugin_path = self.dir.join(&plugin_name).with_extension(PLUGIN_EXTENSION);
        let temporary_path = self.unique_managed_path(&plugin_path, INSTALL_TEMP_PREFIX)?;

        if let Err(error) = tokio::fs::write(&temporary_path, contents).await {
            Self::cleanup_managed_file(&temporary_path);
            return Err(error.into());
        }
        let lifecycle_lock = self.lifecycle_lock(&plugin_name)?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;

        let source = match self.load_library_from_path(&temporary_path, plugin_path.clone()) {
            Ok(source) => source,
            Err(error) => {
                Self::cleanup_managed_file(&temporary_path);
                return Err(error);
            }
        };
        let entry = match source.into_entry(self.options.max_concurrent_calls) {
            Ok(entry) => Arc::new(entry),
            Err(error) => {
                Self::cleanup_managed_file(&temporary_path);
                return Err(error);
            }
        };
        let replacement = match self
            .replace_plugin_file(&temporary_path, &plugin_path)
            .await
        {
            Ok(replacement) => replacement,
            Err(error) => {
                Self::cleanup_managed_file(&temporary_path);
                return Err(error);
            }
        };

        if let Err(error) = self.insert_entry(entry) {
            if let Err(rollback_error) = self.rollback_plugin_file(&plugin_path, replacement).await
            {
                return Err(anyhow!(
                    "failed to register extension: {error}; failed to restore the previous file: {rollback_error}"
                ));
            }
            return Err(error);
        }

        if let Some(backup_path) = replacement.backup_path {
            Self::cleanup_managed_file(&backup_path);
        }

        info!("installed extension {name}");

        Ok(())
    }

    fn load_library(&self, name: &str) -> Result<Source> {
        let plugin_path = self.dir.join(name).with_extension(PLUGIN_EXTENSION);
        self.load_library_from_path(&plugin_path, plugin_path.clone())
    }

    fn load_library_from_path(&self, source_path: &Path, plugin_path: PathBuf) -> Result<Source> {
        let staged_path = self.stage_library(source_path)?;
        info!(
            "load {:?} from {:?}",
            staged_path.display(),
            plugin_path.display()
        );

        #[cfg(target_os = "macos")]
        if let Err(error) = std::process::Command::new("install_name_tool")
            .current_dir(staged_path.parent().unwrap())
            .arg("-id")
            .arg("''")
            .arg(staged_path.file_name().unwrap())
            .output()
        {
            error!("failed to run install_name_tool: {error}");
        }

        let result = (|| -> Result<Source> {
            unsafe {
                let library = Library::new(&staged_path)?;

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
                        "Version mismatch: extension.lib_version={} != tanoshi_lib.lib_version={}",
                        decl.core_version,
                        tanoshi_lib::LIB_VERSION
                    );
                }

                let mut registrar = Source::new(library, decl.rustc_version, decl.core_version)
                    .with_loaded_library_path(staged_path.clone())
                    .with_plugin_path(plugin_path);
                (decl.register)(&mut registrar);

                Ok(registrar)
            }
        })();

        if result.is_err() {
            Self::cleanup_managed_file(&staged_path);
        }
        result
    }

    pub async fn load(&self, name: &str) -> Result<()> {
        let plugin_name = normalize_plugin_name(name);
        let lifecycle_lock = self.lifecycle_lock(&plugin_name)?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;
        let entry = Arc::new(
            self.load_library(&plugin_name)?
                .into_entry(self.options.max_concurrent_calls)?,
        );
        let source_name = entry.source_info.name.to_lowercase();

        if let Some(preferences) =
            tokio::fs::read_to_string(self.dir.join(&source_name).with_extension("json"))
                .await
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        {
            info!("set preferences for {}", source_name);
            self.call_blocking_mut_entry(entry.clone(), "load_preferences", move |extension| {
                extension.set_preferences(preferences)
            })
            .await?;
        }
        self.insert_entry(entry)
    }

    pub async fn insert(&self, source: Source) -> Result<()> {
        let entry = Arc::new(source.into_entry(self.options.max_concurrent_calls)?);
        let lifecycle_lock = self.lifecycle_lock(&entry_plugin_name(&entry))?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;
        self.insert_entry(entry)
    }

    fn insert_entry(&self, entry: Arc<SourceEntry>) -> Result<()> {
        let source_id = entry.source_id;
        let replaced = {
            let mut sources = self.write()?;
            sources.insert(source_id, entry)
        };
        drop(replaced);
        Ok(())
    }

    pub async fn unload(&self, source_id: i64) -> Result<()> {
        loop {
            let entry = match self.read()?.get(&source_id).cloned() {
                Some(entry) => entry,
                None => return Ok(()),
            };
            let plugin_name = entry_plugin_name(&entry);
            let lifecycle_lock = self.lifecycle_lock(&plugin_name)?;
            let _lifecycle_guard = lifecycle_lock.lock_owned().await;
            let mut sources = self.write()?;
            let Some(entry) = sources.get(&source_id).cloned() else {
                return Ok(());
            };
            if entry_plugin_name(&entry) != plugin_name {
                continue;
            }

            let plugin_path = entry.plugin_path().map(Path::to_path_buf);
            if let Some(plugin_path) = plugin_path.as_deref()
                && let Err(error) = std::fs::remove_file(plugin_path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                return Err(anyhow!(
                    "failed to remove extension {}: {error}",
                    plugin_path.display()
                ));
            }

            sources.remove(&source_id);
            drop(sources);
            info!(
                "uninstalled extension {source_id} ({})",
                entry.source_name()
            );
            return Ok(());
        }
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
            Ok(entry.source_info.clone())
        } else {
            println!("Returning dummy source info");
            Ok(dummy_source_info(source_id))
        }
    }

    pub async fn filter_list(&self, source_id: i64) -> Result<Vec<Input>> {
        let entry = self.entry(source_id)?;
        self.call_blocking_entry(
            entry,
            "filter_list",
            |extension| Ok(extension.filter_list()),
        )
        .await
    }

    pub async fn get_preferences(&self, source_id: i64) -> Result<Vec<Input>> {
        let entry = self.entry(source_id)?;
        self.call_blocking_entry(entry, "get_preferences", |extension| {
            extension.get_preferences()
        })
        .await
    }

    pub async fn set_preferences(&self, source_id: i64, preferences: Vec<Input>) -> Result<()> {
        let entry = self.entry(source_id)?;
        let source_name = entry.source_info.name.to_lowercase();
        let extension_preferences = preferences.clone();
        self.call_blocking_mut_entry(entry, "set_preferences", move |extension| {
            extension.set_preferences(extension_preferences)
        })
        .await?;

        tokio::fs::write(
            self.dir.join(source_name).with_extension("json"),
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
        self.call_blocking(source_id, "get_popular_manga", move |extension| {
            extension.get_popular_manga(page)
        })
        .await
    }

    pub async fn get_latest_manga(
        &self,
        source_id: i64,
        page: i64,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        self.call_blocking(source_id, "get_latest_manga", move |extension| {
            extension.get_latest_manga(page)
        })
        .await
    }

    pub async fn search_manga(
        &self,
        source_id: i64,
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        self.call_blocking(source_id, "search_manga", move |extension| {
            extension.search_manga(page, query, filters)
        })
        .await
    }

    pub async fn get_manga_detail(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<tanoshi_lib::prelude::MangaInfo> {
        self.call_blocking(source_id, "get_manga_detail", move |extension| {
            extension.get_manga_detail(path)
        })
        .await
    }

    pub async fn get_chapters(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<tanoshi_lib::prelude::ChapterInfo>> {
        self.call_blocking(source_id, "get_chapters", move |extension| {
            extension.get_chapters(path)
        })
        .await
    }

    pub async fn get_pages(&self, source_id: i64, path: String) -> Result<Vec<String>> {
        self.call_blocking(source_id, "get_pages", move |extension| {
            extension.get_pages(path)
        })
        .await
    }

    pub async fn get_image_bytes(&self, source_id: i64, url: String) -> Result<Bytes> {
        self.call_blocking(source_id, "get_image_bytes", move |extension| {
            extension.get_image_bytes(url)
        })
        .await
    }
}
