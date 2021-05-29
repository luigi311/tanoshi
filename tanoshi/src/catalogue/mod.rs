mod source;
pub use source::{Source, SourceRoot, SourceMutationRoot};

mod manga;
pub use manga::Manga;

mod chapter;
pub use chapter::Chapter;

use crate::context::GlobalContext;

use async_graphql::{Context, Enum, Object, Result};
use tanoshi_lib::extensions::Extension;

/// A type represent sort parameter for query manga from source, normalized across sources
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(remote = "tanoshi_lib::model::SortByParam")]
pub enum SortByParam {
    LastUpdated,
    Title,
    Comment,
    Views,
}

/// A type represent order parameter for query manga from source, normalized across sources
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(remote = "tanoshi_lib::model::SortOrderParam")]
pub enum SortOrderParam {
    Asc,
    Desc,
}

#[derive(Default)]
pub struct CatalogueRoot;

#[Object]
impl CatalogueRoot {
    async fn browse_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "source id")] source_id: i64,
        #[graphql(desc = "keyword of the manga")] keyword: Option<String>,
        #[graphql(desc = "genres of the manga")] genres: Option<Vec<String>>,
        #[graphql(desc = "page")] page: Option<i32>,
        #[graphql(desc = "sort by")] sort_by: Option<SortByParam>,
        #[graphql(desc = "sort order")] sort_order: Option<SortOrderParam>,
    ) -> Result<Vec<Manga>> {
        let sort_by = sort_by.map(|s| s.into());
        let sort_order = sort_order.map(|s| s.into());

        let ctx = ctx.data::<GlobalContext>()?;
        let fetched_manga = {
            let extensions = ctx.extensions.read()?;
            extensions
            .get(source_id)
            .ok_or("no source")?
            .get_mangas(keyword, genres, page, sort_by, sort_order, None)?
        };

        let db = ctx.mangadb.clone();
        let mut manga = vec![];
        for m in fetched_manga {
            let item = if let Some(item) = db.get_manga_by_source_path(source_id, &m.path).await {
                item
            } else {
                let mut item: Manga = m.into();
                let manga_id = db.insert_manga(&item).await?;
                item.id = manga_id;
                item
            };
            manga.push(item);
        }
        Ok(manga)
    }

    async fn manga(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "manga id")] id: i64,
    ) -> Result<Option<Manga>> {
        let ctx = ctx.data::<GlobalContext>()?;
        let db = ctx.mangadb.clone();
        if let Some(mut manga) = db.get_manga_by_id(id).await {
            if manga.incomplete() {
                let m: Manga = {
                    let extensions = ctx.extensions.read()?;
                    extensions
                    .get(manga.source_id)
                    .ok_or("no source")?
                    .get_manga_info(&manga.path)?
                    .into()
                };

                if m.description.is_some() {
                    manga.description = m.description;
                }
                if m.genre.len() > 0 {
                    manga.genre = m.genre;
                }
                if m.author.len() > 0 {
                    manga.author = m.author;
                }
                db.update_manga_info(&manga).await?;
            }
            Ok(Some(manga))
        } else {
            Ok(None)
        }
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
