use bytes::Bytes;
use flume::Sender;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tanoshi_lib::prelude::{Chapter, Extension, ExtensionResult, Filters, Manga, Param, Source};

#[cfg(feature = "compiler")]
use crate::prelude::compile;

// use crate::prelude::ExtensionProxy;

pub type ExtensionResultSender<T> = Sender<ExtensionResult<T>>;

#[derive(Debug)]
pub enum Command {
    Insert(i64, Arc<dyn Extension>),
    Load(String),
    Unload(i64, Sender<()>),
    Exist(i64, Sender<bool>),
    List(Sender<Vec<Source>>),
    Detail(i64, Sender<Source>),
    Filters(i64, ExtensionResultSender<Option<Filters>>),
    GetMangaList(i64, Param, ExtensionResultSender<Vec<Manga>>),
    GetMangaInfo(i64, String, ExtensionResultSender<Manga>),
    GetChapters(i64, String, ExtensionResultSender<Vec<Chapter>>),
    GetPages(i64, String, ExtensionResultSender<Vec<String>>),
}

#[derive(Debug, Clone)]
pub struct ExtensionBus {
    path: PathBuf,
    tx: Sender<Command>,
}

impl ExtensionBus {
    pub fn new<P: AsRef<Path>>(path: P, tx: Sender<Command>) -> Self {
        Self {
            path: PathBuf::new().join(path),
            tx,
        }
    }

    pub fn load(&self) -> Result<(), anyhow::Error> {
        if std::fs::read_dir(&self.path).is_err() {
            let _ = std::fs::create_dir_all(&self.path);
        }

        #[cfg(feature = "compiler")]
        compile(&self.path)?;

        for entry in std::fs::read_dir(&self.path)?.filter_map(|s| s.ok()) {
            if !entry
                .path()
                .extension()
                .map_or(false, |ext| ext == "tanoshi")
            {
                continue;
            }

            let path = entry.path();
            info!("found compiled plugin at {:?}", path.clone());
            self.tx.send(Command::Load(
                path.to_str()
                    .ok_or_else(|| anyhow::anyhow!("no path str"))?
                    .to_string(),
            ))?;
        }

        Ok(())
    }

    pub fn insert(&self, source_id: i64, proxy: Arc<dyn Extension>) -> Result<(), anyhow::Error> {
        Ok(self.tx.send(Command::Insert(source_id, proxy))?)
    }

    pub fn install(&self, name: String, contents: &Bytes) -> Result<(), anyhow::Error> {
        let path = self.path.join(name).with_extension("tanoshi");
        // ExtensionProxy::compile(contents, &path)?;
        std::fs::write(&path, contents)?;

        Ok(self.tx.send(Command::Load(
            path.to_str()
                .ok_or_else(|| anyhow::anyhow!("path can't to string"))?
                .to_string(),
        ))?)
    }

    pub fn unload(&self, source_id: i64) -> Result<(), anyhow::Error> {
        let detail = self.detail(source_id)?;
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::Unload(source_id, tx))?;
        rx.recv_timeout(Duration::from_secs(30))?;
        let path = self.path.join(detail.name).with_extension("tanoshi");
        std::fs::remove_file(path)?;
        Ok(())
    }

    pub fn exist(&self, source_id: i64) -> Result<bool, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::Exist(source_id, tx))?;

        let exist = rx.recv_timeout(Duration::from_secs(30))?;
        Ok(exist)
    }

    pub fn list(&self) -> Result<Vec<Source>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::List(tx))?;

        let sources = rx.recv_timeout(Duration::from_secs(30))?;
        Ok(sources)
    }

    pub fn detail(&self, source_id: i64) -> Result<Source, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::Detail(source_id, tx))?;

        let source = rx.recv_timeout(Duration::from_secs(30))?;
        Ok(source)
    }

    pub fn filters(&self, source_id: i64) -> Result<Option<Filters>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::Filters(source_id, tx))?;

        rx.recv_timeout(Duration::from_secs(30))?
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn get_manga_list(
        &self,
        source_id: i64,
        param: Param,
    ) -> Result<Vec<Manga>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::GetMangaList(source_id, param, tx))?;

        rx.recv_timeout(Duration::from_secs(30))?
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn get_manga_info(&self, source_id: i64, path: String) -> Result<Manga, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::GetMangaInfo(source_id, path, tx))?;

        rx.recv_timeout(Duration::from_secs(30))?
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn get_chapters(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<Chapter>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::GetChapters(source_id, path, tx))?;

        rx.recv_timeout(Duration::from_secs(30))?
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn get_pages(&self, source_id: i64, path: String) -> Result<Vec<String>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::GetPages(source_id, path, tx))?;

        rx.recv_timeout(Duration::from_secs(30))?
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[cfg(feature = "async")]
impl ExtensionBus {
    pub async fn insert_async(
        &self,
        source_id: i64,
        proxy: Arc<dyn Extension>,
    ) -> Result<(), anyhow::Error> {
        Ok(self
            .tx
            .send_async(Command::Insert(source_id, proxy))
            .await?)
    }

    pub async fn install_async(&self, name: String, contents: &Bytes) -> Result<(), anyhow::Error> {
        let path = self.path.join(name).with_extension("tanoshi");
        // ExtensionProxy::compile(contents, &path)?;
        tokio::fs::write(&path, contents).await?;

        Ok(self
            .tx
            .send_async(Command::Load(
                path.to_str()
                    .ok_or_else(|| anyhow::anyhow!("path can't to string"))?
                    .to_string(),
            ))
            .await?)
    }

    pub async fn unload_async(&self, source_id: i64) -> Result<(), anyhow::Error> {
        let detail = self.detail(source_id)?;
        let (tx, rx) = flume::bounded(1);
        self.tx.send(Command::Unload(source_id, tx))?;
        rx.recv_timeout(Duration::from_secs(30))?;
        let path = self.path.join(detail.name).with_extension("tanoshi");
        std::fs::remove_file(path)?;
        Ok(())
    }

    pub async fn exist_async(&self, source_id: i64) -> Result<bool, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send_async(Command::Exist(source_id, tx)).await?;

        let exist = tokio::time::timeout(Duration::from_secs(30), rx.recv_async()).await??;
        Ok(exist)
    }

    pub async fn list_async(&self) -> Result<Vec<Source>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send_async(Command::List(tx)).await?;

        let sources = tokio::time::timeout(Duration::from_secs(30), rx.recv_async()).await??;
        Ok(sources)
    }

    pub async fn detail_async(&self, source_id: i64) -> Result<Source, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send_async(Command::Detail(source_id, tx)).await?;

        let source = tokio::time::timeout(Duration::from_secs(30), rx.recv_async()).await??;
        Ok(source)
    }

    pub async fn filters_async(&self, source_id: i64) -> Result<Option<Filters>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx.send_async(Command::Filters(source_id, tx)).await?;

        tokio::time::timeout(Duration::from_secs(30), rx.recv_async())
            .await??
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub async fn get_manga_list_async(
        &self,
        source_id: i64,
        param: Param,
    ) -> Result<Vec<Manga>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx
            .send_async(Command::GetMangaList(source_id, param, tx))
            .await?;

        tokio::time::timeout(Duration::from_secs(30), rx.recv_async())
            .await??
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub async fn get_manga_info_async(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Manga, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx
            .send_async(Command::GetMangaInfo(source_id, path, tx))
            .await?;

        tokio::time::timeout(Duration::from_secs(30), rx.recv_async())
            .await??
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub async fn get_chapters_async(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<Chapter>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx
            .send_async(Command::GetChapters(source_id, path, tx))
            .await?;

        tokio::time::timeout(Duration::from_secs(30), rx.recv_async())
            .await??
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub async fn get_pages_async(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<String>, anyhow::Error> {
        let (tx, rx) = flume::bounded(1);
        self.tx
            .send_async(Command::GetPages(source_id, path, tx))
            .await?;

        tokio::time::timeout(Duration::from_secs(30), rx.recv_async())
            .await??
            .result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}
