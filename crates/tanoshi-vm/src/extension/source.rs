use crate::api::{BytesToString, Console, Fetch, Print};
use rquickjs::{Context, Function, Object, Promise, Runtime, This};
use tanoshi_lib::models::*;

use anyhow::Result;
use async_trait::async_trait;

pub struct Source(Context, SourceInfo);

impl Source {
    pub fn new(rt: &Runtime, name: &str) -> Result<Self> {
        let ctx = Context::full(rt)?;

        let source = ctx.with(|ctx| -> Result<SourceInfo> {
            let global = ctx.globals();
            global.init_def::<Print>()?;
            global.init_def::<Console>()?;
            global.init_def::<Fetch>()?;
            global.init_def::<BytesToString>()?;

            let module = ctx.compile(
                name,
                format!(
                    "import Source from '{}'; export const s = new Source();",
                    name
                ),
            )?;

            let object = module.get::<_, Object>("s")?;
            global.set("s", object)?;

            Ok(module.get::<_, SourceInfo>("s")?)
        })?;

        info!("{:?}", source);

        Ok(Source(ctx, source))
    }
}

#[async_trait]
impl tanoshi_lib::traits::Extension for Source {
    fn get_source_info(&self) -> SourceInfo {
        self.1.clone()
    }

    fn get_filter_list(&self) -> Result<Vec<Input>> {
        Ok(self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>("getFilterList")?
                .call((This(object.clone()),))
        })?)
    }

    fn get_preferences(&self) -> Result<Vec<Input>> {
        Ok(self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>("getPreferences")?
                .call((This(object.clone()),))
        })?)
    }

    async fn get_popular_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        let promise: Promise<Vec<MangaInfo>> = self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>("getPopularManga")?
                .call((This(object.clone()), page))
        })?;

        Ok(promise.await?)
    }

    async fn get_latest_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        let promise: Promise<Vec<MangaInfo>> = self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>("getLatestManga")?
                .call((This(object.clone()), page))
        })?;

        Ok(promise.await?)
    }

    async fn search_manga(
        &self,
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    ) -> Result<Vec<MangaInfo>> {
        let promise: Promise<Vec<MangaInfo>> = self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object.get::<_, Function>("searchManga")?.call((
                This(object.clone()),
                page,
                query,
                filters,
            ))
        })?;

        Ok(promise.await?)
    }

    async fn get_manga_detail(&self, path: String) -> Result<MangaInfo> {
        let promise: Promise<MangaInfo> = self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>("getMangaDetail")?
                .call((This(object.clone()), path))
        })?;

        Ok(promise.await?)
    }

    async fn get_chapters(&self, path: String) -> Result<Vec<ChapterInfo>> {
        let promise: Promise<Vec<ChapterInfo>> = self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>("getChapters")?
                .call((This(object.clone()), path))
        })?;

        Ok(promise.await?)
    }

    async fn get_pages(&self, path: String) -> Result<Vec<String>> {
        let promise: Promise<Vec<String>> = self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>("getPages")?
                .call((This(object.clone()), path))
        })?;

        Ok(promise.await?)
    }
}
