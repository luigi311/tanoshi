use bytes::Bytes;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tanoshi_lib::prelude::{
    Chapter, Extension, ExtensionResult, Manga, Param, Request, Response, Source,
};
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot::Sender,
    },
    task::JoinHandle,
    time::timeout,
};

#[derive(Debug)]
pub enum Command {
    Insert(i64, Box<dyn Extension>),
    Load(String),
    Unload(i64),
    Exist(i64, Sender<bool>),
    Detail(i64, Sender<Source>),
    GetMangaList(i64, Param, Sender<Vec<Manga>>),
    GetMangaInfo(i64, String, Sender<Manga>),
    GetChapters(i64, String, Sender<Vec<Chapter>>),
    GetPages(i64, String, Sender<Vec<String>>),
}

#[derive(Debug, Clone)]
pub struct ExtensionBus {
    extension_dir_path: String,
    tx: UnboundedSender<Command>,
}

impl ExtensionBus {
    pub fn new(extension_dir_path: String, tx: UnboundedSender<Command>) -> Self {
        Self {
            extension_dir_path,
            tx,
        }
    }

    pub async fn insert(
        &self,
        source_id: i64,
        proxy: Box<dyn Extension>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.tx.send(Command::Insert(source_id, proxy))?)
    }

    pub async fn install(
        &self,
        name: String,
        contents: &Bytes,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from(self.extension_dir_path.clone())
            .join(name)
            .with_extension("wasm");
        std::fs::write(path.clone(), contents)?;

        Ok(self.tx.send(Command::Load(
            path.to_str().ok_or("path can't to string")?.to_string(),
        ))?)
    }

    pub async fn unload(&self, source_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.tx.send(Command::Unload(source_id))?)
    }

    pub async fn exist(&self, source_id: i64) -> Result<bool, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::Exist(source_id, tx))?;

        let exist = timeout(Duration::from_secs(30), rx).await??;
        Ok(exist)
    }

    pub async fn detail(&self, source_id: i64) -> Result<Source, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::Detail(source_id, tx))?;

        let source = timeout(Duration::from_secs(30), rx).await??;
        Ok(source)
    }

    pub async fn get_manga_list(
        &self,
        source_id: i64,
        param: Param,
    ) -> Result<Vec<Manga>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::GetMangaList(source_id, param, tx))?;

        let manga = timeout(Duration::from_secs(30), rx).await??;
        Ok(manga)
    }

    pub async fn get_manga_info(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Manga, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::GetMangaInfo(source_id, path, tx))?;

        let manga = timeout(Duration::from_secs(30), rx).await??;
        Ok(manga)
    }

    pub async fn get_chapters(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<Chapter>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::GetChapters(source_id, path, tx))?;

        let manga = timeout(Duration::from_secs(30), rx).await??;
        Ok(manga)
    }

    pub async fn get_pages(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::GetPages(source_id, path, tx))?;

        let manga = timeout(Duration::from_secs(30), rx).await??;
        Ok(manga)
    }
}
