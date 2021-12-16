use std::{collections::HashMap, sync::Arc};

use super::{Chapter, Source};
use crate::{config::GLOBAL_CONFIG, db::MangaDatabase, user::Claims, utils};
use async_graphql::{
    dataloader::{DataLoader, Loader},
    Context, Object, Result,
};
use chrono::NaiveDateTime;
use tanoshi_vm::extension::SourceManager;

pub type UserFavoriteId = (i64, i64);

pub struct FavoriteLoader {
    pub mangadb: MangaDatabase,
}

#[async_trait::async_trait]
impl Loader<UserFavoriteId> for FavoriteLoader {
    type Value = bool;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserFavoriteId],
    ) -> Result<HashMap<UserFavoriteId, Self::Value>, Self::Error> {
        let user_id = keys
            .iter()
            .next()
            .map(|key| key.0)
            .ok_or_else(|| anyhow::anyhow!("no user id"))?;
        let manga_ids: Vec<i64> = keys.iter().map(|key| key.1).collect();
        let res = self
            .mangadb
            .is_user_library_by_manga_ids(user_id, &manga_ids)
            .await?
            .into_iter()
            .map(|(manga_id, is_library)| ((user_id, manga_id), is_library))
            .collect();
        Ok(res)
    }
}

pub type UserFavoritePath = (i64, String);

#[async_trait::async_trait]
impl Loader<UserFavoritePath> for FavoriteLoader {
    type Value = bool;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserFavoritePath],
    ) -> Result<HashMap<UserFavoritePath, Self::Value>, Self::Error> {
        let user_id = keys
            .iter()
            .next()
            .map(|key| key.0)
            .ok_or_else(|| anyhow::anyhow!("no user id"))?;
        let manga_paths: Vec<String> = keys.iter().map(|key| key.1.clone()).collect();
        let res = self
            .mangadb
            .is_user_library_by_manga_paths(user_id, &manga_paths)
            .await?
            .into_iter()
            .map(|(manga_path, is_library)| ((user_id, manga_path), is_library))
            .collect();
        Ok(res)
    }
}

pub type UserLastReadId = (i64, i64);

pub struct UserLastReadLoader {
    pub mangadb: MangaDatabase,
}

#[async_trait::async_trait]
impl Loader<UserLastReadId> for UserLastReadLoader {
    type Value = NaiveDateTime;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserLastReadId],
    ) -> Result<HashMap<UserLastReadId, Self::Value>, Self::Error> {
        let user_id = keys
            .iter()
            .next()
            .map(|key| key.0)
            .ok_or_else(|| anyhow::anyhow!("no user id"))?;
        let manga_ids: Vec<i64> = keys.iter().map(|key| key.1).collect();
        let res = self
            .mangadb
            .get_last_read_at_by_user_id_and_manga_ids(user_id, &manga_ids)
            .await?
            .into_iter()
            .map(|(manga_id, read_at)| ((user_id, manga_id), read_at))
            .collect();
        Ok(res)
    }
}

pub type UserUnreadChaptersId = (i64, i64);

pub struct UserUnreadChaptersLoader {
    pub mangadb: MangaDatabase,
}

#[async_trait::async_trait]
impl Loader<UserUnreadChaptersId> for UserUnreadChaptersLoader {
    type Value = i64;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserUnreadChaptersId],
    ) -> Result<HashMap<UserUnreadChaptersId, Self::Value>, Self::Error> {
        let user_id = keys
            .iter()
            .next()
            .map(|key| key.0)
            .ok_or_else(|| anyhow::anyhow!("no user id"))?;
        let manga_ids: Vec<i64> = keys.iter().map(|key| key.1).collect();
        let res = self
            .mangadb
            .get_user_library_unread_chapters(user_id, &manga_ids)
            .await?
            .into_iter()
            .map(|(manga_id, count)| ((user_id, manga_id), count))
            .collect();
        Ok(res)
    }
}

/// A type represent manga details, normalized across source
#[derive(Debug, Clone)]
pub struct Manga {
    pub id: i64,
    pub source_id: i64,
    pub title: String,
    pub author: Vec<String>,
    pub genre: Vec<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub path: String,
    pub cover_url: String,
    pub date_added: chrono::NaiveDateTime,
}

impl From<tanoshi_lib::models::MangaInfo> for Manga {
    fn from(m: tanoshi_lib::models::MangaInfo) -> Self {
        Self {
            id: 0,
            source_id: m.source_id,
            title: m.title,
            author: m.author,
            genre: m.genre,
            status: m.status,
            description: m.description,
            path: m.path,
            cover_url: m.cover_url,
            date_added: chrono::NaiveDateTime::from_timestamp(0, 0),
        }
    }
}

impl From<crate::db::model::Manga> for Manga {
    fn from(val: crate::db::model::Manga) -> Self {
        Self {
            id: val.id,
            source_id: val.source_id,
            title: val.title,
            author: val.author,
            genre: val.genre,
            status: val.status,
            description: val.description,
            path: val.path,
            cover_url: val.cover_url,
            date_added: val.date_added,
        }
    }
}

impl From<tanoshi_lib::models::MangaInfo> for crate::db::model::Manga {
    fn from(m: tanoshi_lib::models::MangaInfo) -> Self {
        Self {
            id: 0,
            source_id: m.source_id,
            title: m.title,
            author: m.author,
            genre: m.genre,
            status: m.status,
            description: m.description,
            path: m.path,
            cover_url: m.cover_url,
            date_added: chrono::NaiveDateTime::from_timestamp(0, 0),
        }
    }
}

impl From<Manga> for crate::db::model::Manga {
    fn from(val: Manga) -> Self {
        Self {
            id: val.id,
            source_id: val.source_id,
            title: val.title,
            author: val.author,
            genre: val.genre,
            status: val.status,
            description: val.description,
            path: val.path,
            cover_url: val.cover_url,
            date_added: val.date_added,
        }
    }
}

#[Object]
impl Manga {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn title(&self) -> String {
        self.title.clone()
    }

    async fn author(&self) -> Vec<String> {
        self.author.clone()
    }

    async fn genre(&self) -> Vec<String> {
        self.genre.clone()
    }

    async fn status(&self) -> Option<String> {
        self.status.clone()
    }

    async fn description(&self) -> Option<String> {
        self.description.clone()
    }

    async fn link(&self, ctx: &Context<'_>) -> Result<String> {
        let detail = ctx
            .data::<Arc<SourceManager>>()?
            .get(self.source_id)?
            .get_source_info();
        Ok(format!("{}{}", detail.url, self.path))
    }

    async fn path(&self) -> String {
        self.path.as_str().to_string()
    }

    async fn cover_url(&self) -> Result<String> {
        let secret = &GLOBAL_CONFIG.get().ok_or("secret not set")?.secret;
        Ok(utils::encrypt_url(secret, &self.cover_url)?)
    }

    async fn is_favorite(&self, ctx: &Context<'_>) -> Result<bool> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        let loader = ctx.data::<DataLoader<FavoriteLoader>>()?;
        let is_favorite: Option<bool> = if self.id == 0 {
            loader.load_one((user.sub, self.path.clone())).await?
        } else {
            loader.load_one((user.sub, self.id)).await?
        };

        Ok(is_favorite.unwrap_or(false))
    }

    async fn date_added(&self) -> chrono::NaiveDateTime {
        self.date_added
    }

    async fn unread_chapter_count(&self, ctx: &Context<'_>) -> Result<i64> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;
        let loader = ctx.data::<DataLoader<UserUnreadChaptersLoader>>()?;
        Ok(loader.load_one((user.sub, self.id)).await?.unwrap_or(0))
    }

    async fn last_read_at(&self, ctx: &Context<'_>) -> Result<Option<NaiveDateTime>> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;
        let loader = ctx.data::<DataLoader<UserLastReadLoader>>()?;
        Ok(loader.load_one((user.sub, self.id)).await?)
    }

    async fn source(&self, ctx: &Context<'_>) -> Result<Source> {
        let source = ctx
            .data::<Arc<SourceManager>>()?
            .get(self.source_id)?
            .get_source_info();
        Ok(source.into())
    }

    async fn chapters(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "refresh data from source", default = false)] refresh: bool,
    ) -> Result<Vec<Chapter>> {
        let db = ctx.data::<MangaDatabase>()?;

        if !refresh {
            if let Ok(chapters) = db.get_chapters_by_manga_id(self.id).await {
                return Ok(chapters.into_iter().map(|c| c.into()).collect());
            }
        }

        let chapters: Vec<crate::db::model::Chapter> = ctx
            .data::<Arc<SourceManager>>()?
            .get(self.source_id)?
            .get_chapters(self.path.clone())
            .await?
            .into_iter()
            .map(|c| {
                let mut c: crate::db::model::Chapter = c.into();
                c.manga_id = self.id;
                c
            })
            .collect();

        if chapters.is_empty() {
            return Ok(vec![]);
        }

        db.insert_chapters(&chapters).await?;

        let chapters = db
            .get_chapters_by_manga_id(self.id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<Chapter>>();

        Ok(chapters)
    }

    async fn chapter(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "chapter id")] id: i64,
    ) -> Result<Chapter> {
        let db = ctx.data::<MangaDatabase>()?.clone();
        Ok(db.get_chapter_by_id(id).await?.into())
    }

    async fn next_chapter(&self, ctx: &Context<'_>) -> Result<Option<Chapter>> {
        let db = ctx.data::<MangaDatabase>()?.clone();
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        let mut id = self.id;
        if id == 0 {
            if let Ok(manga) = db
                .get_manga_by_source_path(self.source_id, &self.path)
                .await
            {
                id = manga.id;
            } else {
                return Ok(None);
            }
        }

        Ok(db
            .get_next_chapter_by_manga_id(user.sub, id)
            .await?
            .map(|c| c.into()))
    }
}
