use crate::prelude::Source;
use crate::vm::create_runtime;
use anyhow::anyhow;
use anyhow::Result;
use fnv::FnvHashMap;
use rquickjs::Runtime;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tanoshi_lib::{prelude::SourceInfo, traits::Extension};

pub struct SourceManager {
    dir: PathBuf,
    rt: Runtime,
    extensions: FnvHashMap<i64, Arc<dyn Extension>>,
}

impl SourceManager {
    pub fn new<P: AsRef<Path>>(extension_dir: P) -> Self {
        let rt = create_runtime(&extension_dir).unwrap();

        Self {
            dir: PathBuf::new().join(extension_dir),
            rt,
            extensions: FnvHashMap::default(),
        }
    }

    pub async fn install(&mut self, name: &str, contents: &[u8]) -> Result<SourceInfo> {
        tokio::fs::write(self.dir.join(name).with_extension("mjs"), contents).await?;

        Ok(self.load(name)?)
    }

    pub fn load(&mut self, name: &str) -> Result<SourceInfo> {
        let ext = Arc::new(Source::new(&self.rt, name)?);
        let source_info = ext.get_source_info();
        self.extensions.insert(source_info.id, ext);

        Ok(source_info)
    }

    pub fn insert(&mut self, source: Arc<dyn Extension>) -> Result<()> {
        self.extensions.insert(source.get_source_info().id, source);

        Ok(())
    }

    pub fn unload(&mut self, id: i64) -> Result<Arc<dyn Extension>> {
        Ok(self
            .extensions
            .remove(&id)
            .ok_or(anyhow!("no such source"))?)
    }

    pub async fn remove(&mut self, id: i64) -> Result<()> {
        let source = self.unload(id)?;
        let name = source.get_source_info().name;
        tokio::fs::remove_file(self.dir.join(&name).with_extension("mjs")).await?;

        Ok(())
    }

    pub fn get(&self, id: i64) -> Result<Arc<dyn Extension>> {
        self.extensions
            .get(&id)
            .cloned()
            .ok_or(anyhow!("source not exists"))
    }

    pub fn list(&self) -> Result<Vec<SourceInfo>> {
        Ok(self
            .extensions
            .iter()
            .map(|(_, ext)| ext.get_source_info())
            .collect())
    }
}
