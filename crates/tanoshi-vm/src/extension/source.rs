use std::sync::RwLock;

use anyhow::{Result, anyhow};
use libloading::Library;
use once_cell::sync::OnceCell;
use tanoshi_lib::prelude::{Extension, SourceInfo};

/// A loaded source that can outlive its entry in the manager's source map.
pub struct SourceEntry {
    pub(crate) source_id: i64,
    pub(crate) source_info: SourceInfo,
    // Keep the extension before the library. Struct fields are dropped in
    // declaration order, so the extension is destroyed before its library.
    pub(crate) extension: RwLock<Box<dyn Extension>>,
    #[allow(dead_code)]
    pub(crate) lib: Option<Library>,
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

    pub(crate) fn source_info(&self) -> Result<SourceInfo> {
        self.with_extension(|extension| Ok(extension.get_source_info()))
    }

    pub(crate) fn source_name(&self) -> &str {
        &self.source_info.name
    }
}

pub struct Source {
    pub(crate) extension: OnceCell<Box<dyn Extension>>,
    #[allow(dead_code)]
    pub(crate) lib: Option<Library>,
    pub rustc_version: String,
    pub lib_version: String,
}

impl Source {
    pub fn new(lib: Library, rustc_version: &str, lib_version: &str) -> Source {
        Source {
            lib: Some(lib),
            rustc_version: rustc_version.to_string(),
            lib_version: lib_version.to_string(),
            extension: OnceCell::new(),
        }
    }

    pub fn from(extension: Box<dyn Extension>) -> Self {
        Self {
            lib: None,
            rustc_version: tanoshi_lib::RUSTC_VERSION.to_string(),
            lib_version: tanoshi_lib::LIB_VERSION.to_string(),
            extension: OnceCell::from(extension),
        }
    }

    pub(crate) fn into_entry(self) -> Result<SourceEntry> {
        let Self {
            extension,
            lib,
            rustc_version,
            lib_version,
        } = self;
        let extension = extension
            .into_inner()
            .ok_or_else(|| anyhow!("extension not initiated"))?;
        let source_info = extension.get_source_info();
        let source_id = source_info.id;

        Ok(SourceEntry {
            source_id,
            source_info,
            extension: RwLock::new(extension),
            lib,
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
