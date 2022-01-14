use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use tanoshi_lib::prelude::{Input, SourceInfo};
use tokio::sync::oneshot;

use crate::{
    prelude::Source,
    vm::{self, SourceCommand, PLUGIN_EXTENSION},
};

#[derive(Clone)]
pub struct SourceBus {
    dir: PathBuf,
    tx: flume::Sender<SourceCommand>,
}

impl SourceBus {
    pub fn new<P: AsRef<Path>>(extension_dir: P) -> Self {
        let (tx, rx) = flume::unbounded();
        vm::run(&extension_dir, rx);
        Self {
            dir: PathBuf::new().join(extension_dir),
            tx,
        }
    }

    pub async fn load_all(&self) -> Result<()> {
        let mut read_dir = tokio::fs::read_dir(&self.dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let mut name = format!("{:?}", entry.file_name());
            name.remove(0);
            name.remove(name.len() - 1);
            if name.ends_with(PLUGIN_EXTENSION) {
                self.load(&name).await?;
            }
        }
        Ok(())
    }

    pub async fn exists(&self, source_id: i64) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::Exists(source_id, tx))
            .await?;

        Ok(rx.await?)
    }

    pub async fn list(&self) -> Result<Vec<SourceInfo>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send_async(SourceCommand::List(tx)).await?;

        Ok(rx.await?)
    }

    pub async fn install(&self, repo_url: &str, name: &str) -> Result<()> {
        let source_file_url = format!(
            "{}/{}/{}.{}",
            repo_url,
            env!("TARGET"),
            name.to_lowercase(),
            PLUGIN_EXTENSION
        );

        info!("downloading {}", source_file_url);

        let contents = reqwest::get(&source_file_url).await?.bytes().await?;

        tokio::fs::write(
            self.dir
                .join(&name.to_lowercase())
                .with_extension(PLUGIN_EXTENSION),
            contents,
        )
        .await?;

        let source = vm::load(&self.dir, &name.to_lowercase())?;
        self.insert(source).await
    }

    pub async fn load(&self, name: &str) -> Result<()> {
        let mut source = vm::load(&self.dir, name)?;

        let source_name = source
            .extension
            .get()
            .ok_or(anyhow!("not initiated"))?
            .get_source_info()
            .name
            .to_lowercase();

        if let Some(preferences) =
            tokio::fs::read_to_string(self.dir.join(source_name).with_extension("json"))
                .await
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        {
            info!("set preferences");
            source
                .extension
                .get_mut()
                .ok_or(anyhow!("not initiated"))?
                .set_preferences(preferences)?;
        }
        self.insert(source).await
    }

    pub async fn insert(&self, source: Source) -> Result<()> {
        self.tx.send_async(SourceCommand::Insert(source)).await?;
        Ok(())
    }

    pub async fn unload(&self, source_id: i64) -> Result<()> {
        self.tx.send_async(SourceCommand::Unload(source_id)).await?;
        Ok(())
    }

    pub async fn remove(&self, source_id: i64) -> Result<()> {
        self.unload(source_id).await
    }

    pub async fn get_source_info(&self, source_id: i64) -> Result<SourceInfo> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::GetSourceInfo(source_id, tx))
            .await?;

        rx.await?
    }

    pub async fn filter_list(&self, source_id: i64) -> Result<Vec<Input>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::GetFilterList(source_id, tx))
            .await?;

        rx.await?
    }

    pub async fn get_preferences(&self, source_id: i64) -> Result<Vec<Input>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::GetPreferences(source_id, tx))
            .await?;
        rx.await?
    }

    pub async fn set_preferences(&self, source_id: i64, preferences: Vec<Input>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::SetPreferences(
                source_id,
                preferences.clone(),
                tx,
            ))
            .await?;
        let _ = rx.await?;

        let source_info = self.get_source_info(source_id).await?;
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
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::GetPopularManga(source_id, page, tx))
            .await?;
        rx.await?
    }

    pub async fn get_latest_manga(
        &self,
        source_id: i64,
        page: i64,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::GetLatestManga(source_id, page, tx))
            .await?;
        rx.await?
    }

    pub async fn search_manga(
        &self,
        source_id: i64,
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::SearchManga(
                source_id, page, query, filters, tx,
            ))
            .await?;
        rx.await?
    }

    pub async fn get_manga_detail(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<tanoshi_lib::prelude::MangaInfo> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::GetMangaDetail(source_id, path, tx))
            .await?;
        rx.await?
    }

    pub async fn get_chapters(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<tanoshi_lib::prelude::ChapterInfo>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::GetChapters(source_id, path, tx))
            .await?;
        rx.await?
    }

    pub async fn get_pages(&self, source_id: i64, path: String) -> Result<Vec<String>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send_async(SourceCommand::GetPages(source_id, path, tx))
            .await?;
        rx.await?
    }
}
