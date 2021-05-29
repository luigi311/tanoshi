use crate::context::GlobalContext;
use async_graphql::{Context, Object, Result, SimpleObject};
use serde::Deserialize;
use tanoshi_lib::extensions::Extension;

#[derive(Debug, Clone, Deserialize)]
pub struct SourceIndex {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub rustc_version: String,
    pub core_version: String,
    pub version: String,
}

impl Into<Source> for SourceIndex {
    fn into(self) -> Source {
        Source {
            id: self.id,
            name: self.name.clone(),
            version: self.version.clone(),
            icon: "".to_string(),
            need_login: false,
        }
    }
}

#[derive(Clone, SimpleObject)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub icon: String,
    pub need_login: bool,
}

impl From<tanoshi_lib::model::Source> for Source {
    fn from(s: tanoshi_lib::model::Source) -> Self {
        Self {
            id: s.id,
            name: s.name,
            version: s.version,
            icon: s.icon,
            need_login: s.need_login,
        }
    }
}

#[derive(Default)]
pub struct SourceRoot;

#[Object]
impl SourceRoot {
    async fn installed_sources(&self, ctx: &Context<'_>) -> Result<Vec<Source>> {
        let extensions = ctx.data::<GlobalContext>()?.extensions.read()?;
        Ok(extensions
            .extentions()
            .iter()
            .map(|(_, ext)| ext.detail().into())
            .collect())
    }

    async fn available_sources(&self, ctx: &Context<'_>) -> Result<Vec<Source>> {
        let url = format!(
            "https://raw.githubusercontent.com/faldez/tanoshi-extension-dev/repo-{}/index.json",
            std::env::consts::OS
        );
        let source_indexes = reqwest::get(url).await?.json::<Vec<SourceIndex>>().await?;
        let extensions = ctx.data::<GlobalContext>()?.extensions.read()?;
        let exts = extensions.extentions();
        let mut sources: Vec<Source> = vec![];
        for index in source_indexes {
            if exts.get(&index.id).is_none() {
                sources.push(index.into());
            }
        }
        Ok(sources)
    }

    async fn source(&self, ctx: &Context<'_>, source_id: i64) -> Result<Option<Source>> {
        let exts = ctx.data::<GlobalContext>()?.extensions.read()?;
        Ok(exts
            .extentions()
            .get(&source_id)
            .map(|ext| ext.detail().into()))
    }
}

#[derive(Default)]
pub struct SourceMutationRoot;

#[Object]
impl SourceMutationRoot {
    async fn install_source(&self, ctx: &Context<'_>, source_id: i64) -> Result<i64> {
        let url = format!(
            "https://raw.githubusercontent.com/faldez/tanoshi-extension-dev/repo-{}/index.json",
            std::env::consts::OS
        );

        // let exts = &ctx.data::<GlobalContext>()?.extensions.read()?;
        // if exts.extentions().get(&source_id).is_some() {
        //     return Err("source installed, use updateSource to update".into());
        // }

        let source_indexes = reqwest::get(url).await?.json::<Vec<SourceIndex>>().await?;
        let mut source: Option<SourceIndex> = None;
        for index in source_indexes {
            if index.id == source_id {
                source = Some(index);
            }
        }

        if let Some(source) = source {
            let url = format!(
                "https://raw.githubusercontent.com/faldez/tanoshi-extension-dev/repo-{}/{}",
                std::env::consts::OS,
                source.path,
            );
            let raw = reqwest::get(url).await?.bytes().await?;

            Ok(source.id)
        } else {
            Err("Not found".into())
        }
    }

    async fn uninstall_source(&self, ctx: &Context<'_>, source_id: i64) -> Result<i64> {
        todo!()
    }
}
