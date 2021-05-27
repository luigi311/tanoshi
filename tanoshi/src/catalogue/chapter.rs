use super::{Manga, Source};
use crate::{context::GlobalContext, user};
use async_graphql::{Context, Object, Result};
use chrono::NaiveDateTime;

/// A type represent chapter, normalized across source
pub struct Chapter {
    pub id: i64,
    pub source_id: i64,
    pub manga_id: i64,
    pub title: String,
    pub path: String,
    pub rank: i64,
    pub prev: Option<i64>,
    pub next: Option<i64>,
    pub read_at: Option<chrono::NaiveDateTime>,
    pub uploaded: chrono::NaiveDateTime,
    pub date_added: chrono::NaiveDateTime,
    pub last_page_read: Option<i64>,
    pub pages: Vec<String>,
}

impl From<tanoshi_lib::model::Chapter> for Chapter {
    fn from(ch: tanoshi_lib::model::Chapter) -> Self {
        Self {
            id: 0,
            source_id: ch.source_id,
            manga_id: 0,
            title: ch.title,
            path: ch.path,
            rank: ch.rank,
            prev: None,
            next: None,
            read_at: None,
            uploaded: ch.uploaded,
            date_added: chrono::NaiveDateTime::from_timestamp(chrono::Local::now().timestamp(), 0),
            last_page_read: None,
            pages: vec![],
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

    async fn rank(&self) -> i64 {
        self.rank
    }

    async fn prev(&self) -> Option<i64> {
        self.prev
    }

    async fn next(&self) -> Option<i64> {
        self.next
    }

    async fn read_at(&self, ctx: &Context<'_>) -> Result<Option<chrono::NaiveDateTime>> {
        let user = user::get_claims(ctx).ok_or("no token")?;
        let read_at = ctx
            .data_unchecked::<GlobalContext>()
            .mangadb
            .get_user_history_read_at(user.sub, self.id)
            .await?;

        Ok(read_at)
    }

    async fn uploaded(&self) -> NaiveDateTime {
        self.uploaded.clone()
    }

    async fn date_added(&self) -> chrono::NaiveDateTime {
        self.date_added
    }

    async fn last_page_read(&self, ctx: &Context<'_>) -> Result<Option<i64>> {
        let user = user::get_claims(ctx).ok_or("no token")?;
        let last_page = ctx
            .data_unchecked::<GlobalContext>()
            .mangadb
            .get_user_history_last_read(user.sub, self.id)
            .await?;

        Ok(last_page)
    }

    async fn source(&self, ctx: &Context<'_>) -> Source {
        let ext = ctx
            .data_unchecked::<GlobalContext>()
            .extensions
            .get(self.source_id)
            .unwrap();
        Source {
            id: ext.detail().id,
            name: ext.detail().name.clone(),
            version: ext.detail().version.clone(),
            icon: ext.detail().icon.clone(),
            need_login: ext.detail().need_login,
        }
    }

    async fn manga(&self, ctx: &Context<'_>) -> Manga {
        ctx.data_unchecked::<GlobalContext>()
            .mangadb
            .get_manga_by_id(self.manga_id)
            .await
            .unwrap()
    }

    async fn pages(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "fetch from source", default = false)] fetch: bool,
    ) -> Result<Vec<String>> {
        info!("pages: {}, fetch: {}", self.pages.len(), fetch);
        if !self.pages.is_empty() && !fetch {
            return Ok(self.pages.clone());
        }

        let pages = fetch_pages(
            ctx,
            self.path.clone(),
            self.source_id.clone(),
            self.id.clone(),
        )
        .await?;

        ctx.data_unchecked::<GlobalContext>()
        .mangadb
        .update_page_by_chapter_id(self.id, &pages)
        .await?;

        Ok(pages)
    }
}

async fn fetch_pages(
    ctx: &Context<'_>,
    path: String,
    source_id: i64,
    chapter_id: i64,
) -> anyhow::Result<Vec<String>> {
    let pages = ctx
        .data_unchecked::<GlobalContext>()
        .extensions
        .get(source_id)
        .unwrap()
        .get_pages(path.clone())
        .await
        .unwrap_or_else(|e| {
            log::error!("{} for {}", e, (&chapter_id).clone());
            vec![]
        });

    Ok(pages)
}
