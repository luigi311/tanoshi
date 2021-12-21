use crate::api::{BytesToString, Console, Fetch, Print};
use rquickjs::{Context, Function, Object, Promise, Runtime, This};
use tanoshi_lib::models::*;

use anyhow::Result;
use async_trait::async_trait;

macro_rules! call_js {
    ($self:ident, $name:literal $(,$arg:ident)*) => {
        $self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>($name)?
                .call((This(object), ($($arg,)*)))
        })?
    };
}

pub struct Source(Context, SourceInfo);

impl Source {
    pub fn new(rt: &Runtime, name: &str) -> Result<Self> {
        let ctx = Context::full(rt)?;

        let mut source = ctx.with(|ctx| -> Result<SourceInfo> {
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
        if let Lang::Single(lang) = &source.languages {
            if lang == "all" {
                source.languages = Lang::All;
            }
        }
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
        let filter = call_js!(self, "getFilterList");
        Ok(filter)
    }

    fn get_preferences(&self) -> Result<Vec<Input>> {
        let prefs = call_js!(self, "getPreferences");
        Ok(prefs)
    }

    async fn get_popular_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        let promise: Promise<Vec<MangaInfo>> = call_js!(self, "getPopularManga", page);
        Ok(promise.await?)
    }

    async fn get_latest_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        let promise: Promise<Vec<MangaInfo>> = self.0.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>("getLatestManga")?
                .call((This(object), page))
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
            object
                .get::<_, Function>("searchManga")?
                .call((This(object), page, query, filters))
        })?;

        Ok(promise.await?)
    }

    async fn get_manga_detail(&self, path: String) -> Result<MangaInfo> {
        let promise: Promise<MangaInfo> = call_js!(self, "getMangaDetail", path);

        Ok(promise.await?)
    }

    async fn get_chapters(&self, path: String) -> Result<Vec<ChapterInfo>> {
        let promise: Promise<Vec<ChapterInfo>> = call_js!(self, "getChapters", path);

        Ok(promise.await?)
    }

    async fn get_pages(&self, path: String) -> Result<Vec<String>> {
        let promise: Promise<Vec<String>> = call_js!(self, "getPages", path);

        Ok(promise.await?)
    }
}
