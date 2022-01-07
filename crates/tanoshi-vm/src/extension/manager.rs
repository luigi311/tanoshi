use crate::{prelude::Source, vm::create_runtime};
use anyhow::{anyhow, Result};
use fnv::FnvHashMap;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use tanoshi_lib::{
    prelude::{Input, SourceInfo},
    traits::Extension,
};

#[derive(Clone)]
pub struct SourceManager {
    dir: PathBuf,
    extensions: Arc<RwLock<FnvHashMap<i64, Arc<dyn Extension>>>>,
}

impl SourceManager {
    pub fn new<P: AsRef<Path>>(extension_dir: P) -> Self {
        Self {
            dir: PathBuf::new().join(extension_dir),
            extensions: Arc::new(RwLock::new(FnvHashMap::default())),
        }
    }

    fn read(&self) -> Result<RwLockReadGuard<FnvHashMap<i64, Arc<dyn Extension>>>> {
        self.extensions
            .read()
            .map_err(|e| anyhow!("failed to lock: {}", e))
    }

    fn write(&self) -> Result<RwLockWriteGuard<FnvHashMap<i64, Arc<dyn Extension>>>> {
        self.extensions
            .write()
            .map_err(|e| anyhow!("failed to lock: {}", e))
    }

    fn read_preferences(&self, source_name: &str) -> Result<Vec<Input>> {
        let path = self.dir.join(source_name).with_extension(".json");
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    pub fn set_preferences(&self, source_id: i64, preferences: Vec<Input>) -> Result<()> {
        let source_info = self.get(source_id)?.get_source_info();
        let path = self.dir.join(&source_info.name).with_extension(".json");

        let contents = serde_json::to_string(&preferences)?;
        std::fs::write(path, contents)?;

        self.read()?
            .get(&source_id)
            .ok_or(anyhow!("no such source"))?
            .set_preferences(preferences)?;

        Ok(())
    }

    pub async fn install(&self, name: &str, contents: &[u8]) -> Result<SourceInfo> {
        tokio::fs::write(self.dir.join(name).with_extension("mjs"), contents).await?;

        Ok(self.load(name)?)
    }

    pub fn load(&self, name: &str) -> Result<SourceInfo> {
        let rt = create_runtime(&self.dir)?;
        let ext = Arc::new(Source::new(rt, name)?);
        let source_info = ext.get_source_info();
        if let Ok(preferences) = self.read_preferences(&source_info.name) {
            ext.set_preferences(preferences)?;
        }
        self.insert(ext)?;
        Ok(source_info)
    }

    pub fn insert(&self, source: Arc<dyn Extension>) -> Result<()> {
        self.write()?.insert(source.get_source_info().id, source);

        Ok(())
    }

    pub fn unload(&self, id: i64) -> Result<Arc<dyn Extension>> {
        self.write()?.remove(&id).ok_or(anyhow!("no such source"))
    }

    pub async fn remove(&self, id: i64) -> Result<()> {
        let source = self.unload(id)?;
        let name = source.get_source_info().name;
        tokio::fs::remove_file(self.dir.join(&name).with_extension("mjs")).await?;

        Ok(())
    }

    pub fn get(&self, id: i64) -> Result<Arc<dyn Extension>> {
        self.read()?
            .get(&id)
            .cloned()
            .ok_or(anyhow!("source not exists"))
    }

    pub fn list(&self) -> Result<Vec<SourceInfo>> {
        Ok(self
            .read()?
            .iter()
            .map(|(_, ext)| ext.get_source_info())
            .collect())
    }
}
