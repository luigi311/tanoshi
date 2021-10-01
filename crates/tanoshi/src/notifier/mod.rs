pub mod pushover;
pub mod telegram;

use crate::{user, worker::Command as WorkerCommand};
use async_graphql::{Context, Object, Result};
use tokio::sync::mpsc::Sender;

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
        ctx.data::<Sender<WorkerCommand>>()?
            .send(WorkerCommand::TelegramMessage(
                chat_id,
                "Test Notification".to_string(),
            ))
            .await?;

        Ok(true)
    }

    async fn test_pushover(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "pushover user key")] user_key: String,
    ) -> Result<bool> {
        let _ = user::get_claims(ctx)?;
        ctx.data::<Sender<WorkerCommand>>()?
            .send(WorkerCommand::PushoverMessage {
                user_key,
                title: None,
                body: "Test Notification".to_string(),
            })
            .await?;

        Ok(true)
    }
}
