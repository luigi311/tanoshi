use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use pathdiff::diff_paths;
use rquickjs::{
    BuiltinLoader, BuiltinResolver, FileResolver, ModuleLoader, NativeLoader, Runtime,
    ScriptLoader, Tokio,
};
use tanoshi_lib::prelude::{ChapterInfo, Extension, Input, MangaInfo, SourceInfo};
use tokio::sync::oneshot;

pub enum Command {
    GetSourceInfo(oneshot::Sender<SourceInfo>),
    GetFilterList(oneshot::Sender<Result<Vec<Input>>>),
    GetPreferences(oneshot::Sender<Result<Vec<Input>>>),
    SetPreferences(Vec<Input>, oneshot::Sender<Result<()>>),
    GetPopularManga(i64, oneshot::Sender<Result<Vec<MangaInfo>>>),
    GetLatestManga(i64, oneshot::Sender<Result<Vec<MangaInfo>>>),
    SearchManga(
        i64,
        Option<String>,
        Option<Vec<Input>>,
        oneshot::Sender<Result<Vec<MangaInfo>>>,
    ),
    GetMangaDetail(String, oneshot::Sender<Result<MangaInfo>>),
    GetChapters(String, oneshot::Sender<Result<Vec<ChapterInfo>>>),
    GetPages(String, oneshot::Sender<Result<Vec<String>>>),
}

pub fn create_runtime<P: AsRef<Path>>(extension_dir: P) -> Result<Runtime> {
    let extension_dir =
        if let Some(relative_path) = diff_paths(&extension_dir, env::current_dir().unwrap()) {
            relative_path
        } else {
            PathBuf::new().join(extension_dir)
        };

    let rt = Runtime::new()?;

    let resolver = (
        BuiltinResolver::default(),
        FileResolver::default()
            .with_path(
                extension_dir
                    .to_str()
                    .ok_or(anyhow!("failed to convert path to_str"))?,
            )
            .with_pattern("{}.mjs")
            .with_native(),
    );

    let loader = (
        BuiltinLoader::default(),
        ModuleLoader::default(),
        ScriptLoader::default().with_extension("mjs"),
        NativeLoader::default(),
    );

    rt.set_loader(resolver, loader);
    rt.spawn_executor(Tokio);

    Ok(rt)
}

#[derive(Clone)]
pub struct SourceBus(pub(crate) flume::Sender<Command>, pub(crate) SourceInfo);

impl SourceBus {
    pub fn new<E>(source: E) -> Self
    where
        E: Extension,
    {
        let (tx, rx) = flume::unbounded();
        let source_info = source.get_source_info();

        run(rx, source);

        Self(tx, source_info)
    }

    pub fn get_source_info(&self) -> SourceInfo {
        self.1.clone()
    }

    pub async fn get_filter_list(&self) -> Result<Vec<Input>> {
        let (tx, rx) = oneshot::channel();
        self.0.send(Command::GetFilterList(tx)).unwrap();
        rx.await.unwrap()
    }

    pub async fn get_preferences(&self) -> Result<Vec<Input>> {
        let (tx, rx) = oneshot::channel();
        self.0.send(Command::GetPreferences(tx)).unwrap();
        rx.await.unwrap()
    }

    pub async fn set_preferences(&self, preferences: Vec<Input>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0
            .send(Command::SetPreferences(preferences, tx))
            .unwrap();
        rx.await.unwrap()
    }

    pub async fn get_popular_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        let (tx, rx) = oneshot::channel();
        self.0.send(Command::GetPopularManga(page, tx)).unwrap();
        rx.await.unwrap()
    }

    pub async fn get_latest_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        let (tx, rx) = oneshot::channel();
        self.0.send(Command::GetLatestManga(page, tx)).unwrap();
        rx.await.unwrap()
    }

    pub async fn search_manga(
        &self,
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    ) -> Result<Vec<MangaInfo>> {
        let (tx, rx) = oneshot::channel();
        self.0
            .send(Command::SearchManga(page, query, filters, tx))
            .unwrap();
        rx.await.unwrap()
    }

    pub async fn get_manga_detail(&self, path: String) -> Result<MangaInfo> {
        let (tx, rx) = oneshot::channel();
        self.0.send(Command::GetMangaDetail(path, tx)).unwrap();
        rx.await.unwrap()
    }

    pub async fn get_chapters(&self, path: String) -> Result<Vec<ChapterInfo>> {
        let (tx, rx) = oneshot::channel();
        self.0.send(Command::GetChapters(path, tx)).unwrap();
        rx.await.unwrap()
    }

    pub async fn get_pages(&self, path: String) -> Result<Vec<String>> {
        let (tx, rx) = oneshot::channel();
        self.0.send(Command::GetPages(path, tx)).unwrap();
        rx.await.unwrap()
    }
}

pub(crate) fn run<E>(rx: flume::Receiver<Command>, source: E)
where
    E: Extension,
{
    tokio::spawn(async move {
        while let Ok(cmd) = rx.recv_async().await {
            match cmd {
                Command::GetSourceInfo(tx) => {
                    let _ = tx.send(source.get_source_info());
                }
                Command::GetFilterList(tx) => {
                    let _ = tx.send(source.get_preferences());
                }
                Command::GetPreferences(tx) => {
                    let _ = tx.send(source.get_preferences());
                }
                Command::SetPreferences(preferences, tx) => {
                    let _ = tx.send(source.set_preferences(preferences));
                }
                Command::GetPopularManga(page, tx) => {
                    let res = source.get_popular_manga(page).await;
                    let _ = tx.send(res);
                }
                Command::GetLatestManga(page, tx) => {
                    let res = source.get_latest_manga(page).await;
                    let _ = tx.send(res);
                }
                Command::SearchManga(page, query, filters, tx) => {
                    let res = source.search_manga(page, query, filters).await;
                    let _ = tx.send(res);
                }
                Command::GetMangaDetail(path, tx) => {
                    let res = source.get_manga_detail(path).await;
                    let _ = tx.send(res);
                }
                Command::GetChapters(path, tx) => {
                    let res = source.get_chapters(path).await;
                    let _ = tx.send(res);
                }
                Command::GetPages(path, tx) => {
                    let res = source.get_pages(path).await;
                    let _ = tx.send(res);
                }
            }
        }
    });
}
