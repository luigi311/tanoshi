use std::{collections::HashMap, path::PathBuf, sync::Arc};

use super::{Manga, Source};
use crate::{config::GLOBAL_CONFIG, db::MangaDatabase, local, user::Claims, utils};
use async_graphql::{
    dataloader::{DataLoader, Loader},
    Context, Object, Result, SimpleObject,
};
use chrono::NaiveDateTime;
use tanoshi_vm::extension::SourceBus;

#[derive(Debug, Clone, SimpleObject)]
pub struct ReadProgress {
    pub at: NaiveDateTime,
    pub last_page: i64,
    pub is_complete: bool,
}

impl From<crate::db::model::ReadProgress> for ReadProgress {
    fn from(val: crate::db::model::ReadProgress) -> Self {
        Self {
            at: val.at,
            last_page: val.last_page,
            is_complete: val.is_complete,
        }
    }
}

pub type UserHistoryId = (i64, i64);

pub struct ReadProgressLoader {
    pub mangadb: MangaDatabase,
}

#[async_trait::async_trait]
impl Loader<UserHistoryId> for ReadProgressLoader {
    type Value = ReadProgress;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserHistoryId],
    ) -> Result<HashMap<UserHistoryId, Self::Value>, Self::Error> {
        let user_id = keys
            .iter()
            .next()
            .map(|key| key.0)
            .ok_or_else(|| anyhow::anyhow!("no user id"))?;
        let chapter_ids: Vec<i64> = keys.iter().map(|key| key.1).collect();
        let res = self
            .mangadb
            .get_user_history_progress_by_chapter_ids(user_id, &chapter_ids)
            .await?
            .into_iter()
            .map(|(chapter_id, progress)| ((user_id, chapter_id), progress.into()))
            .collect();
        Ok(res)
    }
}

pub type ChapterId = i64;

pub struct PrevChapterLoader {
    pub mangadb: MangaDatabase,
}

#[async_trait::async_trait]
impl Loader<ChapterId> for PrevChapterLoader {
    type Value = i64;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[ChapterId],
    ) -> Result<HashMap<ChapterId, Self::Value>, Self::Error> {
        let res = self.mangadb.get_prev_chapter_id_by_ids(keys).await?;
        Ok(res)
    }
}

pub type MangaId = i64;

pub struct MangaLoader {
    pub mangadb: MangaDatabase,
}

#[async_trait::async_trait]
impl Loader<MangaId> for MangaLoader {
    type Value = Manga;

    type Error = Arc<anyhow::Error>;

    async fn load(&self, keys: &[ChapterId]) -> Result<HashMap<MangaId, Self::Value>, Self::Error> {
        let res = self
            .mangadb
            .get_manga_by_ids(keys)
            .await?
            .into_iter()
            .map(|m| (m.id, m.into()))
            .collect();
        Ok(res)
    }
}

pub struct NextChapterLoader {
    pub mangadb: MangaDatabase,
}

#[async_trait::async_trait]
impl Loader<ChapterId> for NextChapterLoader {
    type Value = i64;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[ChapterId],
    ) -> Result<HashMap<ChapterId, Self::Value>, Self::Error> {
        let res = self.mangadb.get_next_chapter_id_by_ids(keys).await?;
        Ok(res)
    }
}

/// A type represent chapter, normalized across source
#[derive(Debug, Clone)]
pub struct Chapter {
    pub id: i64,
    pub source_id: i64,
    pub manga_id: i64,
    pub title: String,
    pub path: String,
    pub number: f64,
    pub scanlator: String,
    pub uploaded: chrono::NaiveDateTime,
    pub date_added: chrono::NaiveDateTime,
    pub read_progress: Option<ReadProgress>,
    pub downloaded_path: Option<String>,
}

impl From<tanoshi_lib::models::ChapterInfo> for Chapter {
    fn from(ch: tanoshi_lib::models::ChapterInfo) -> Self {
        Self {
            id: 0,
            source_id: ch.source_id,
            manga_id: 0,
            title: ch.title,
            path: ch.path,
            number: ch.number,
            scanlator: ch.scanlator.unwrap_or_default(),
            uploaded: chrono::NaiveDateTime::from_timestamp(ch.uploaded, 0),
            date_added: chrono::NaiveDateTime::from_timestamp(chrono::Local::now().timestamp(), 0),
            read_progress: None,
            downloaded_path: None,
        }
    }
}

impl From<crate::db::model::Chapter> for Chapter {
    fn from(val: crate::db::model::Chapter) -> Self {
        Self {
            id: val.id,
            source_id: val.source_id,
            manga_id: val.manga_id,
            title: val.title,
            path: val.path,
            number: val.number,
            scanlator: val.scanlator,
            uploaded: val.uploaded,
            date_added: val.date_added,
            read_progress: None,
            downloaded_path: val.downloaded_path,
        }
    }
}

impl From<tanoshi_lib::models::ChapterInfo> for crate::db::model::Chapter {
    fn from(ch: tanoshi_lib::models::ChapterInfo) -> Self {
        Self {
            id: 0,
            source_id: ch.source_id,
            manga_id: 0,
            title: ch.title,
            path: ch.path,
            number: ch.number,
            scanlator: ch.scanlator.unwrap_or_default(),
            uploaded: chrono::NaiveDateTime::from_timestamp(ch.uploaded, 0),
            date_added: chrono::NaiveDateTime::from_timestamp(chrono::Local::now().timestamp(), 0),
            downloaded_path: None,
        }
    }
}

impl From<Chapter> for crate::db::model::Chapter {
    fn from(val: Chapter) -> Self {
        Self {
            id: val.id,
            source_id: val.source_id,
            manga_id: val.manga_id,
            title: val.title,
            path: val.path,
            number: val.number,
            scanlator: val.scanlator,
            uploaded: val.uploaded,
            date_added: val.date_added,
            downloaded_path: val.downloaded_path,
        }
    }
}

#[Object]
impl Chapter {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn title(&self) -> String {
        self.title.clone()
    }

    async fn path(&self) -> String {
        self.path.clone()
    }

    async fn number(&self) -> f64 {
        self.number
    }

    async fn scanlator(&self) -> String {
        self.scanlator.clone()
    }

    async fn prev(&self, ctx: &Context<'_>) -> Result<Option<i64>> {
        let loader = ctx.data::<DataLoader<PrevChapterLoader>>()?;
        Ok(loader.load_one(self.id).await?)
    }

    async fn next(&self, ctx: &Context<'_>) -> Result<Option<i64>> {
        let loader = ctx.data::<DataLoader<NextChapterLoader>>()?;
        Ok(loader.load_one(self.id).await?)
    }

    async fn read_progress(&self, ctx: &Context<'_>) -> Result<Option<ReadProgress>> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        let loader = ctx.data::<DataLoader<ReadProgressLoader>>()?;
        Ok(loader.load_one((user.sub, self.id)).await?)
    }

    async fn uploaded(&self) -> NaiveDateTime {
        self.uploaded
    }

    async fn date_added(&self) -> chrono::NaiveDateTime {
        self.date_added
    }

    async fn source(&self, ctx: &Context<'_>) -> Result<Source> {
        let source = ctx.data::<SourceBus>()?.get_source_info(self.source_id)?;
        Ok(source.into())
    }

    async fn manga(&self, ctx: &Context<'_>) -> Result<Manga> {
        let loader = ctx.data::<DataLoader<MangaLoader>>()?;
        loader
            .load_one(self.manga_id)
            .await?
            .ok_or_else(|| "manga not found".into())
    }

    async fn pages(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "fetch from source", default = false)] _fetch: bool,
        #[graphql(desc = "encrypt url", default = true)] encrypt: bool,
    ) -> Result<Vec<String>> {
        let pages = if let Some(downloaded_path) =
            self.downloaded_path.clone().map(|p| PathBuf::new().join(p))
        {
            tokio::task::spawn_blocking(move || local::get_pages_from_archive(&downloaded_path))
                .await??
        } else {
            ctx.data::<SourceBus>()?
                .get_pages(self.source_id, self.path.clone())
                .await?
        };

        if !encrypt {
            return Ok(pages);
        }

        let secret = &GLOBAL_CONFIG.get().ok_or("secret not set")?.secret;
        let pages = pages
            .iter()
            .map(|page| utils::encrypt_url(secret, page).unwrap_or_default())
            .collect();

        Ok(pages)
    }

    async fn downloaded_path(&self) -> Option<String> {
        self.downloaded_path.clone()
    }
}
