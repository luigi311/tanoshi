use super::{common::InputList, guard::AdminGuard};
use crate::{
    domain::services::source::SourceService,
    infrastructure::{
        auth::Claims, config::GLOBAL_CONFIG, domain::repositories::source::SourceRepositoryImpl,
    },
};
use async_graphql::{Context, Object, Result};
use serde::Deserialize;
use tanoshi_vm::extension::SourceBus;

#[derive(Clone, Deserialize)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub version: String,
    pub rustc_version: String,
    pub lib_version: String,
    pub icon: String,
    #[serde(default)]
    pub has_update: bool,
}

impl From<crate::domain::entities::source::Source> for Source {
    fn from(s: crate::domain::entities::source::Source) -> Self {
        Self {
            id: s.id,
            name: s.name.to_string(),
            url: s.url.to_string(),
            version: s.version.to_string(),
            rustc_version: "".to_string(),
            lib_version: "".to_string(),
            icon: s.icon.to_string(),
            has_update: false,
        }
    }
}

#[Object]
impl Source {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn name(&self) -> String {
        self.name.clone()
    }

    async fn url(&self) -> String {
        self.url.clone()
    }

    async fn version(&self) -> String {
        self.version.clone()
    }

    async fn icon(&self) -> String {
        self.icon.clone()
    }

    async fn has_update(&self) -> bool {
        self.has_update
    }

    async fn filters(&self, ctx: &Context<'_>) -> Result<InputList> {
        let filters = ctx.data::<SourceBus>()?.filter_list(self.id)?;

        Ok(InputList(filters))
    }

    async fn preferences(&self, ctx: &Context<'_>) -> Result<InputList> {
        let preferences = ctx.data::<SourceBus>()?.get_preferences(self.id)?;

        Ok(InputList(preferences))
    }
}

#[derive(Default)]
pub struct SourceRoot;

#[Object]
impl SourceRoot {
    async fn installed_sources(
        &self,
        ctx: &Context<'_>,
        check_update: bool,
    ) -> Result<Vec<Source>> {
        let _ = ctx.data::<Claims>()?;

        let repo_url = GLOBAL_CONFIG
            .get()
            .map(|cfg| &cfg.extension_repository)
            .ok_or("no config set")?;

        let sources = ctx
            .data::<SourceService<SourceRepositoryImpl>>()?
            .get_installed_sources(repo_url, check_update)
            .await?
            .into_iter()
            .map(Source::from)
            .collect();

        Ok(sources)
    }

    async fn available_sources(&self, ctx: &Context<'_>) -> Result<Vec<Source>> {
        let _ = ctx.data::<Claims>()?;

        let repo_url = GLOBAL_CONFIG
            .get()
            .map(|cfg| &cfg.extension_repository)
            .ok_or("no config set")?;

        let sources = ctx
            .data::<SourceService<SourceRepositoryImpl>>()?
            .get_available_sources(&repo_url)
            .await?
            .into_iter()
            .map(Source::from)
            .collect();

        Ok(sources)
    }

    async fn source(&self, ctx: &Context<'_>, source_id: i64) -> Result<Source> {
        let _ = ctx.data::<Claims>()?;

        let source = ctx
            .data::<SourceService<SourceRepositoryImpl>>()?
            .get_source_by_id(source_id)
            .await?
            .into();

        Ok(source)
    }
}

#[derive(Default)]
pub struct SourceMutationRoot;

#[Object]
impl SourceMutationRoot {
    #[graphql(guard = "AdminGuard::new()")]
    async fn install_source(&self, ctx: &Context<'_>, source_id: i64) -> Result<i64> {
        if ctx.data::<SourceBus>()?.exists(source_id).await? {
            return Err("source installed, use updateSource to update".into());
        }

        let repo_url = GLOBAL_CONFIG
            .get()
            .map(|cfg| &cfg.extension_repository)
            .ok_or("no config set")?;

        ctx.data::<SourceService<SourceRepositoryImpl>>()?
            .install_source(&repo_url, source_id)
            .await?;

        Ok(source_id)
    }

    #[graphql(guard = "AdminGuard::new()")]
    async fn uninstall_source(&self, ctx: &Context<'_>, source_id: i64) -> Result<i64> {
        ctx.data::<SourceService<SourceRepositoryImpl>>()?
            .uninstall_source(source_id)
            .await?;

        Ok(source_id)
    }

    #[graphql(guard = "AdminGuard::new()")]
    async fn update_source(&self, ctx: &Context<'_>, source_id: i64) -> Result<i64> {
        let repo_url = GLOBAL_CONFIG
            .get()
            .map(|cfg| &cfg.extension_repository)
            .ok_or("no config set")?;

        ctx.data::<SourceService<SourceRepositoryImpl>>()?
            .update_source(&repo_url, source_id)
            .await?;

        Ok(source_id)
    }

    #[graphql(guard = "AdminGuard::new()")]
    async fn set_preferences(
        &self,
        ctx: &Context<'_>,
        source_id: i64,
        preferences: InputList,
    ) -> Result<i64> {
        ctx.data::<SourceBus>()?
            .set_preferences(source_id, preferences.0)
            .await?;

        Ok(source_id)
    }
}