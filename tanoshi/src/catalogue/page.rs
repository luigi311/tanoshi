use async_graphql::{Context, Object, Result, Schema, Subscription, ID};
use chrono::NaiveDateTime;
use crate::context::GlobalContext;

/// A type represent chapter, normalized across source
pub struct Page {
    pub id: i64,
    pub source_id: i64,
    pub manga_id: i64,
    pub chapter_id: i64,
    pub rank: i64,
    pub url: String,
    pub read_at: Option<chrono::NaiveDateTime>,
    pub date_added: chrono::NaiveDateTime,
}

impl From<tanoshi_lib::model::Page> for Page {
    fn from(p: tanoshi_lib::model::Page) -> Self {
        Page {
            id: 0,
            source_id: 0,
            manga_id: 0,
            chapter_id: 0,
            rank: p.rank,
            url: p.url.clone(),
            read_at: None,
            date_added: NaiveDateTime::from_timestamp(chrono::Local::now().timestamp(), 0),
        }
    }
}

#[Object]
impl Page {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn source_id(&self) -> i64 {
        self.source_id
    }

    async fn manga_id(&self) -> i64 {
        self.manga_id
    }

    async fn chapter_id(&self) -> i64 {
        self.chapter_id
    }

    async fn rank(&self) -> i64 {
        self.rank
    }

    async fn url(&self) -> String {
        self.url.clone()
    }

    async fn read_at(&self) -> Option<chrono::NaiveDateTime> {
        self.read_at
    }

    async fn date_added(&self) -> chrono::NaiveDateTime {
        self.date_added
    }
}

