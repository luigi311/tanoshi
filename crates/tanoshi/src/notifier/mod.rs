pub mod pushover;
pub mod telegram;

use crate::{notifier::pushover::Pushover, user};
use async_graphql::{Context, Object, Result};
use teloxide::{
    adaptors::{AutoSend, DefaultParseMode},
    prelude::Requester,
    Bot,
};

#[derive(Default)]
pub struct NotificationRoot;

#[Object]
impl NotificationRoot {
    async fn test_telegram(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "telegram chat id")] chat_id: i64,
    ) -> Result<bool> {
        let _ = user::get_claims(ctx)?;
        ctx.data::<DefaultParseMode<AutoSend<Bot>>>()?
            .send_message(chat_id, "Test Notification")
            .await?;

        Ok(true)
    }

    async fn test_pushover(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "pushover user key")] user_key: String,
    ) -> Result<bool> {
        let _ = user::get_claims(ctx)?;
        ctx.data::<Pushover>()?
            .send_notification(&user_key, "Test Notification")
            .await?;

        Ok(true)
    }
}
