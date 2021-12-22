mod source;

pub use source::{Source, SourceMutationRoot, SourceRoot};

pub mod manga;
pub use manga::Manga;

pub mod chapter;
pub use chapter::Chapter;
use tanoshi_vm::extension::SourceManager;

use crate::db::MangaDatabase;

use async_graphql::{scalar, Context, Object, Result};
use tanoshi_lib::models::Input;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct InputList(Vec<Input>);

scalar!(InputList);

#[derive(Default)]
pub struct CatalogueRoot;

#[Object]
impl CatalogueRoot {
    async fn get_popular_manga(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "source id")] source_id: i64,
        #[graphql(desc = "page")] page: i64,
    ) -> Result<Vec<Manga>> {
        let fetched_manga = ctx
            .data::<SourceManager>()?
            .get(source_id)?
            .get_popular_manga(page)
            .await?
            .into_iter()
            .map(Manga::from)
            .collect();

        Ok(fetched_manga)
    }
    async fn get_latest_manga(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "source id")] source_id: i64,
        #[graphql(desc = "page")] page: i64,
    ) -> Result<Vec<Manga>> {
        let fetched_manga = ctx
            .data::<SourceManager>()?
            .get(source_id)?
            .get_latest_manga(page)
            .await?
            .into_iter()
            .map(Manga::from)
            .collect();

        Ok(fetched_manga)
    }

    async fn browse_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "source id")] source_id: i64,
        #[graphql(desc = "page")] page: i64,
        #[graphql(desc = "query")] query: Option<String>,
        #[graphql(desc = "filters")] filters: Option<InputList>,
    ) -> Result<Vec<Manga>> {
        let fetched_manga = ctx
            .data::<SourceManager>()?
            .get(source_id)?
            .search_manga(page, query, filters.map(|filters| filters.0))
            .await?
            .into_iter()
            .map(Manga::from)
            .collect();

        Ok(fetched_manga)
    }

    async fn manga_by_source_path(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "source id")] source_id: i64,
        #[graphql(desc = "path to manga in source")] path: String,
    ) -> Result<Manga> {
        let db = ctx.data::<MangaDatabase>()?;

        let manga = if let Ok(manga) = db.get_manga_by_source_path(source_id, &path).await {
            manga
        } else {
            let mut m: crate::db::model::Manga = ctx
                .data::<SourceManager>()?
                .get(source_id)?
                .get_manga_detail(path)
                .await?
                .into();

            db.insert_manga(&mut m).await?;
            m
        };

        Ok(manga.into())
    }

    async fn manga(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "manga id")] id: i64,
        #[graphql(desc = "refresh data from source", default = false)] refresh: bool,
    ) -> Result<Manga> {
        let db = ctx.data::<MangaDatabase>()?;
        let manga = db.get_manga_by_id(id).await?;
        if refresh {
            let mut m: crate::db::model::Manga = ctx
                .data::<SourceManager>()?
                .get(manga.source_id)?
                .get_manga_detail(manga.path.clone())
                .await?
                .into();

            m.id = manga.id;

            db.insert_manga(&mut m).await?;

            Ok(m.into())
        } else {
            Ok(manga.into())
        }
    }

    async fn chapter(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "chapter id")] id: i64,
    ) -> Result<Chapter> {
        let db = ctx.data::<MangaDatabase>()?;
        Ok(db.get_chapter_by_id(id).await?.into())
    }
}
