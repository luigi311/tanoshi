use std::time::Duration;

use super::Chapter;
use crate::context::GlobalContext;
use futures::{stream, StreamExt};
use async_graphql::{Context, Object};

/// A type represent manga details, normalized across source
#[derive(Debug)]
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
    pub last_read_chapter: Option<i32>,
    pub is_favorite: bool,
    pub date_added: chrono::NaiveDateTime,
}

impl Manga {
    pub fn incomplete(&self) -> bool {
        self.status.is_none()
            || self.description.is_none()
            || self.author.is_empty()
            || self.genre.is_empty()
    }
}

impl From<tanoshi_lib::model::Manga> for Manga {
    fn from(m: tanoshi_lib::model::Manga) -> Self {
        Self {
            id: 0,
            source_id: m.source_id,
            title: m.title.clone(),
            author: m.author.clone(),
            genre: m.genre.clone(),
            status: m.status.clone(),
            description: m.description.clone(),
            path: m.path.clone(),
            cover_url: m.cover_url.clone(),
            last_read_chapter: None,
            is_favorite: false,
            date_added: chrono::NaiveDateTime::from_timestamp(0, 0),
        }
    }
}

#[Object]
impl Manga {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn source_id(&self) -> i64 {
        self.source_id
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

    async fn path(&self) -> String {
        self.path.as_str().to_string()
    }

    async fn cover_url(&self) -> String {
        self.cover_url.as_str().to_string()
    }

    async fn last_read_chapter(&self) -> Option<i32> {
        self.last_read_chapter
    }

    async fn is_favorite(&self) -> bool {
        self.is_favorite
    }

    async fn date_added(&self) -> chrono::NaiveDateTime {
        self.date_added
    }

    async fn chapters(&self, ctx: &Context<'_>) -> Vec<Chapter> {
        let manga_id = self.id.clone();
        let db = ctx.data_unchecked::<GlobalContext>().db.clone();
        match db.get_chapters_by_manga_id(manga_id).await {
            Ok(chapters) => chapters,
            Err(_) => {
                let chapters = ctx.data_unchecked::<GlobalContext>()
                    .extensions
                    .get(self.source_id)
                    .unwrap()
                    .get_chapters(self.path.clone())
                    .await
                    .unwrap();

                let chapter_stream = stream::iter(chapters);
                let chapter_stream = chapter_stream.then(|chapter| async {
                    match db
                        .get_chapter_by_source_path(chapter.source_id, &chapter.path)
                        .await
                    {
                        Some(ch) => ch,
                        None => {
                            let mut ch: Chapter = chapter.into();
                            ch.manga_id = (&manga_id).clone();
                            let id = db.insert_chapter(&ch).await.unwrap();
                            ch.id = id;

                            tokio::time::sleep(Duration::from_millis(1)).await;
                            return ch;
                        }
                    }
                });
                chapter_stream.collect().await
            }
        }
    }

    async fn chapter(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "chapter id")] id: i64,
    ) -> Option<Chapter> {
        let db = ctx.data_unchecked::<GlobalContext>().db.clone();
        db.get_chapter_by_id(id).await
    }
}
