use async_graphql::{Context, Object, Result};
use chrono::NaiveDateTime;

pub struct RecentChapter {
    pub manga_id: i64,
    pub chapter_id: i64,
    pub manga_title: String,
    pub cover_url: String,
    pub chapter_title: String,
    pub read_at: NaiveDateTime,
    pub last_page_read: i64,
}

#[Object]
impl RecentChapter {
    async fn manga_id(&self) -> i64 {
        self.manga_id
    }

    async fn chapter_id(&self) -> i64 {
        self.chapter_id
    }

    async fn manga_title(&self) -> String {
        self.manga_title.clone()
    }

    async fn cover_url(&self) -> String {
        self.cover_url.clone()
    }

    async fn chapter_title(&self) -> String {
        self.chapter_title.clone()
    }

    async fn read_at(&self) -> NaiveDateTime {
        self.read_at
    }

    async fn last_page_read(&self) -> i64 {
        self.last_page_read
    }
}

pub struct RecentUpdate {
    pub manga_id: i64,
    pub chapter_id: i64,
    pub manga_title: String,
    pub cover_url: String,
    pub chapter_title: String,
    pub uploaded: NaiveDateTime,
}

#[Object]
impl RecentUpdate {
    async fn manga_id(&self) -> i64 {
        self.manga_id
    }

    async fn chapter_id(&self) -> i64 {
        self.chapter_id
    }

    async fn manga_title(&self) -> String {
        self.manga_title.clone()
    }

    async fn cover_url(&self) -> String {
        self.cover_url.clone()
    }

    async fn chapter_title(&self) -> String {
        self.chapter_title.clone()
    }

    async fn uploaded(&self) -> NaiveDateTime {
        self.uploaded
    }
}