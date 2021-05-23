use crate::context::GlobalContext;
use async_graphql::{Context, Object};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SourceIndex {
    pub name: String,
    pub path: String,
    pub rustc_version: String,
    pub core_version: String,
    pub version: String,
}

#[derive(Clone)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub icon: String,
    pub need_login: bool,
}

#[Object]
impl Source {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn name(&self) -> String {
        self.name.clone()
    }


    async fn version(&self) -> String {
        self.version.clone()
    }

    async fn icon(&self) -> String {
        self.icon.clone()
    }

    async fn need_login(&self) -> bool {
        self.need_login
    }
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
    async fn installed_sources(&self, ctx: &Context<'_>) -> Vec<Source> {
        let exts = ctx
            .data_unchecked::<GlobalContext>()
            .extensions
            .extentions();
        exts.iter().map(|(_, ext)| ext.detail().into()).collect()
    }

    async fn source(&self, ctx: &Context<'_>, source_id: i64) -> Option<Source> {
        let exts = ctx
            .data_unchecked::<GlobalContext>()
            .extensions
            .extentions();
        exts.iter().find(|(id, _)| **id == source_id).map(|(_, ext)| ext.detail().into())
    }
}
