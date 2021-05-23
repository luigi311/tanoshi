use super::{Manga, Page, Source};
use crate::context::GlobalContext;
use crate::db::Db;
use futures::{stream, StreamExt};
use async_graphql::{Context, Object, Result, Schema, Subscription, ID};
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

    async fn read_at(&self) -> Option<chrono::NaiveDateTime> {
        self.read_at
    }

    async fn uploaded(&self) -> NaiveDateTime {
        self.uploaded.clone()
    }

    async fn date_added(&self) -> chrono::NaiveDateTime {
        self.date_added
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
            need_login: ext.detail().need_login
        }
    }

    async fn manga(&self, ctx: &Context<'_>) -> Manga {
        ctx.data_unchecked::<GlobalContext>()
            .db
            .get_manga_by_id(self.manga_id)
            .await
            .unwrap()
    }

    async fn pages(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "fetch from source", default = false)] fetch: bool,
    ) -> Vec<Page> {
        let source_id = self.source_id.clone();
        let manga_id = self.manga_id.clone();
        let chapter_id = self.id.clone();
        let db = ctx.data_unchecked::<GlobalContext>().db.clone();

        let mut result = if !fetch {
            db.get_pages_by_chapter_id(chapter_id).await
        } else {
            fetch_pages(
                ctx.data_unchecked::<GlobalContext>(),
                self.path.clone(),
                source_id,
                manga_id,
                chapter_id,
            )
            .await
        };
        if let Err(_) = result {
            result = fetch_pages(
                ctx.data_unchecked::<GlobalContext>(),
                self.path.clone(),
                source_id,
                manga_id,
                chapter_id,
            )
            .await;
        }
        result.unwrap_or(vec![])
    }
}

async fn fetch_pages(
    ctx: &GlobalContext,
    path: String,
    source_id: i64,
    manga_id: i64,
    chapter_id: i64,
) -> anyhow::Result<Vec<Page>> {
    let db = ctx.db.clone();
    let pages = ctx
        .extensions
        .get(source_id)
        .unwrap()
        .get_pages(path.clone())
        .await
        .unwrap_or_else(|e| {
            log::error!("{} for {}", e, (&chapter_id).clone());
            vec![]
        });
    let page_stream = stream::iter(pages);
    let page_stream = page_stream.then(|page| async {
        match db
            .get_page_by_source_url((&source_id).clone(), &page.url)
            .await
        {
            Some(page) => page,
            None => {
                let mut page: Page = page.into();
                page.source_id = (&source_id).clone();
                page.manga_id = (&manga_id).clone();
                page.chapter_id = (&chapter_id).clone();
                let id = db.insert_page(&page).await.unwrap_or(0);
                page.id = id;
                page
            }
        }
    });

    Ok(page_stream.collect().await)
}
