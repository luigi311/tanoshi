use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use anyhow::{Result, anyhow};
use libloading::Library;
use once_cell::sync::OnceCell;
use tanoshi_lib::prelude::{Extension, SourceInfo};
use tokio::sync::Semaphore;

pub(crate) struct LoadedLibrary {
    library: Option<Library>,
    path: Option<PathBuf>,
}

impl LoadedLibrary {
    fn new(library: Library) -> Self {
        Self {
            library: Some(library),
            path: None,
        }
    }
}

impl Drop for LoadedLibrary {
    fn drop(&mut self) {
        let library = self.library.take();
        drop(library);

        if let Some(path) = self.path.take()
            && let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!(
                "failed to remove staged extension library {}: {error}; retrying at startup",
                path.display()
            );
        }
    }
}

/// A loaded source that can outlive its entry in the manager's source map.
pub struct SourceEntry {
    pub(crate) source_id: i64,
    pub(crate) source_info: SourceInfo,
    // Keep the extension before the library. Struct fields are dropped in
    // declaration order, so the extension is destroyed before its library.
    pub(crate) extension: RwLock<Box<dyn Extension>>,
    pub(crate) limiter: Arc<Semaphore>,
    #[allow(dead_code)]
    pub(crate) library: Option<LoadedLibrary>,
    pub(crate) plugin_path: Option<PathBuf>,
    pub(crate) rustc_version: String,
    pub(crate) lib_version: String,
}

impl SourceEntry {
    pub(crate) fn with_extension<T>(
        &self,
        call: impl FnOnce(&dyn Extension) -> Result<T>,
    ) -> Result<T> {
        let extension = self
            .extension
            .read()
            .map_err(|error| anyhow!("source {} read lock is poisoned: {error}", self.source_id))?;
        call(extension.as_ref())
    }

    pub(crate) fn with_extension_mut<T>(
        &self,
        call: impl FnOnce(&mut dyn Extension) -> Result<T>,
    ) -> Result<T> {
        let mut extension = self.extension.write().map_err(|error| {
            anyhow!("source {} write lock is poisoned: {error}", self.source_id)
        })?;
        call(extension.as_mut())
    }

    pub(crate) fn source_name(&self) -> &str {
        &self.source_info.name
    }

    pub(crate) fn plugin_path(&self) -> Option<&Path> {
        self.plugin_path.as_deref()
    }
}

pub struct Source {
    pub(crate) extension: OnceCell<Box<dyn Extension>>,
    pub(crate) library: Option<LoadedLibrary>,
    pub(crate) plugin_path: Option<PathBuf>,
    pub rustc_version: String,
    pub lib_version: String,
}

impl Source {
    pub fn new(lib: Library, rustc_version: &str, lib_version: &str) -> Source {
        Source {
            library: Some(LoadedLibrary::new(lib)),
            plugin_path: None,
            rustc_version: rustc_version.to_string(),
            lib_version: lib_version.to_string(),
            extension: OnceCell::new(),
        }
    }

    pub fn from(extension: Box<dyn Extension>) -> Self {
        Self {
            library: None,
            plugin_path: None,
            rustc_version: tanoshi_lib::RUSTC_VERSION.to_string(),
            lib_version: tanoshi_lib::LIB_VERSION.to_string(),
            extension: OnceCell::from(extension),
        }
    }

    pub(crate) fn with_loaded_library_path(mut self, path: PathBuf) -> Self {
        if let Some(library) = self.library.as_mut() {
            library.path = Some(path);
        }
        self
    }

    pub(crate) fn with_plugin_path(mut self, path: PathBuf) -> Self {
        self.plugin_path = Some(path);
        self
    }

    pub(crate) fn into_entry(self, max_concurrent_calls: usize) -> Result<SourceEntry> {
        let source_info = self
            .extension
            .get()
            .ok_or_else(|| anyhow!("extension not initiated"))?
            .get_source_info();
        let Self {
            extension,
            library,
            plugin_path,
            rustc_version,
            lib_version,
        } = self;
        let extension = extension
            .into_inner()
            .ok_or_else(|| anyhow!("extension not initiated"))?;
        let source_id = source_info.id;

        Ok(SourceEntry {
            source_id,
            source_info,
            extension: RwLock::new(extension),
            limiter: Arc::new(Semaphore::new(max_concurrent_calls)),
            library,
            plugin_path,
            rustc_version,
            lib_version,
        })
    }
}

impl tanoshi_lib::extensions::PluginRegistrar for Source {
    fn register_function(&mut self, extension: Box<dyn Extension>) {
        self.extension
            .set(extension)
            .map_err(|_| "extension already initiated")
            .unwrap();
    }
}
