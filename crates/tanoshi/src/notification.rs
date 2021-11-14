use crate::{notifier::Notifier, user};
use async_graphql::{Context, Object, Result};

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
        ctx.data::<Notifier>()?
            .send_message_to_telegram(chat_id, "Test Notification")
            .await?;

        Ok(true)
    }

    async fn test_pushover(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "pushover user key")] user_key: String,
    ) -> Result<bool> {
        let _ = user::get_claims(ctx)?;
        ctx.data::<Notifier>()?
            .send_message_to_pushover(&user_key, None, "Test Notification")
            .await?;

        Ok(true)
    }
}
