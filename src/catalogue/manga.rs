use super::Chapter;
use crate::{context::GlobalContext, user};
use async_graphql::{Context, Object, Result};

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
    pub date_added: chrono::NaiveDateTime,
}

impl From<&tanoshi_lib::data::Manga> for Manga {
    fn from(m: &tanoshi_lib::data::Manga) -> Self {
        m.clone().into()
    }
}

impl From<tanoshi_lib::data::Manga> for Manga {
    fn from(m: tanoshi_lib::data::Manga) -> Self {
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

    async fn is_favorite(&self, ctx: &Context<'_>) -> Result<bool> {
        let user = user::get_claims(ctx).ok_or("no token")?;
        let mangadb = &ctx.data_unchecked::<GlobalContext>().mangadb;

        let mut id = self.id;
        if id == 0 {
            if let Some(manga) = mangadb
                .get_manga_by_source_path(self.source_id, &self.path)
                .await
            {
                id = manga.id;
            } else {
                return Ok(false);
            }
        }

        if let Ok(fav) = mangadb.is_user_library(user.sub, id).await {
            Ok(fav)
        } else {
            Err("error query".into())
        }
    }

    async fn date_added(&self) -> chrono::NaiveDateTime {
        self.date_added
    }

    async fn chapters(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "refresh data from source", default = false)] refresh: bool,
    ) -> Result<Vec<Chapter>> {
        let manga_id = self.id;
        let ctx = ctx.data::<GlobalContext>()?;
        let db = ctx.mangadb.clone();

        if !refresh {
            if let Ok(chapters) = db.get_chapters_by_manga_id(manga_id).await {
                return Ok(chapters);
            }
        }

        let chapters: Vec<Chapter> = {
            let extensions = ctx.extensions.clone();
            extensions
                .get_chapters(self.source_id, self.path.clone())
                .await?
                .into_iter()
                .map(|c| {
                    let mut c: Chapter = c.into();
                    c.manga_id = self.id;
                    c
                })
                .collect()
        };

        db.insert_chapters(&chapters).await?;

        Ok(db.get_chapters_by_manga_id(manga_id).await?)
    }

    async fn chapter(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "chapter id")] id: i64,
    ) -> Option<Chapter> {
        let db = ctx.data_unchecked::<GlobalContext>().mangadb.clone();
        db.get_chapter_by_id(id).await
    }
}
