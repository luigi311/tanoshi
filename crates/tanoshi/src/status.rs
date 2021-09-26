use std::sync::Arc;

use async_graphql::{Context, Object, Result, SimpleObject};

use crate::context::GlobalContext;

#[derive(Debug, SimpleObject)]
struct Status {
    activated: bool,
    version: String,
}

#[derive(Default)]
pub struct StatusRoot;

#[Object]
impl StatusRoot {
    async fn server_status(&self, ctx: &Context<'_>) -> Result<Status> {
        let activated = ctx
            .data::<Arc<GlobalContext>>()?
            .userdb
            .get_users_count()
            .await?
            > 0;
        let version = env!("CARGO_PKG_VERSION").to_string();

        Ok(Status { activated, version })
    }
}
