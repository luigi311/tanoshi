use std::collections::HashMap;

use crate::api::{BytesToString, Console, Fetch, Print};
use rquickjs::{Context, Function, Object, Promise, Runtime, This};
use tanoshi_lib::models::*;

use anyhow::Result;
use async_trait::async_trait;

macro_rules! call_js {
    ($self:ident, $name:literal $(,$arg:ident)*) => {
        $self.ctx.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            object
                .get::<_, Function>($name)?
                .call((This(object), $($arg,)*))
        })?
    };
}

pub struct Source {
    ctx: Context,
    info: SourceInfo,
    headers: HashMap<String, String>,
    filter_list: Vec<Input>,
}

impl Source {
    pub fn new(rt: &Runtime, name: &str) -> Result<Self> {
        let ctx = Context::full(rt)?;

        let (mut info, headers, filter_list) = ctx.with(
            |ctx| -> Result<(SourceInfo, HashMap<String, String>, Vec<Input>)> {
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

                let info = module.get::<_, SourceInfo>("s")?;
                let headers = object
                    .get::<_, Function>("headers")?
                    .call((This(object.clone()),))?;
                let filter_list = object
                    .get::<_, Function>("filterList")?
                    .call((This(object.clone()),))?;

                global.set("s", object)?;

                Ok((info, headers, filter_list))
            },
        )?;

        if let Lang::Single(lang) = &info.languages {
            if lang == "all" {
                info.languages = Lang::All;
            }
        }

        info!("{:?} {:?}", info, headers);

        Ok(Source {
            ctx,
            info,
            headers,
            filter_list,
        })
    }
}

#[async_trait]
impl tanoshi_lib::traits::Extension for Source {
    fn get_source_info(&self) -> SourceInfo {
        self.info.clone()
    }

    fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    fn filter_list(&self) -> Vec<Input> {
        self.filter_list.clone()
    }

    fn get_preferences(&self) -> Result<Vec<Input>> {
        self.ctx.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            Ok(object.get::<_, Vec<Input>>("preferences")?)
        })
    }

    fn set_preferences(&self, preferences: Vec<Input>) -> Result<()> {
        self.ctx.with(|ctx| {
            let object = ctx.globals().get::<_, Object>("s")?;
            Ok(object.set("preferences", preferences)?)
        })
    }

    async fn get_popular_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        let promise: Promise<Vec<MangaInfo>> = call_js!(self, "getPopularManga", page);
        Ok(promise.await?)
    }

    async fn get_latest_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        let promise: Promise<Vec<MangaInfo>> = call_js!(self, "getLatestManga", page);

        Ok(promise.await?)
    }

    async fn search_manga(
        &self,
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    ) -> Result<Vec<MangaInfo>> {
        let promise: Promise<Vec<MangaInfo>> = call_js!(self, "searchManga", page, query, filters);

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
