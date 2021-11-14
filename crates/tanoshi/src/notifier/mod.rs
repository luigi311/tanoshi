use teloxide::{
    adaptors::{AutoSend, DefaultParseMode},
    prelude::Requester,
    Bot,
};

use self::pushover::Pushover;
use crate::db::UserDatabase;

pub mod pushover;
pub mod telegram;

pub type Telegram = DefaultParseMode<AutoSend<Bot>>;

pub struct Builder {
    userdb: UserDatabase,
    pushover: Option<Pushover>,
    telegram: Option<Telegram>,
}
impl Builder {
    pub fn new(userdb: UserDatabase) -> Self {
        Self {
            userdb,
            pushover: None,
            telegram: None,
        }
    }

    pub fn telegram(self, telegram: Telegram) -> Self {
        Self {
            telegram: Some(telegram),
            ..self
        }
    }

    pub fn pushover(self, pushover: Pushover) -> Self {
        Self {
            pushover: Some(pushover),
            ..self
        }
    }

    pub fn build(self) -> Notifier {
        Notifier {
            userdb: self.userdb,
            telegram: self.telegram,
            pushover: self.pushover,
        }
    }
}

pub struct Notifier {
    userdb: UserDatabase,
    pushover: Option<Pushover>,
    telegram: Option<Telegram>,
}

impl Notifier {
    pub async fn send_all(
        &self,
        user_id: i64,
        title: Option<String>,
        body: &str,
    ) -> Result<(), anyhow::Error> {
        let user = self.userdb.get_user_by_id(user_id).await?;
        if let Some(user_key) = user.pushover_user_key {
            let _ = self.send_message_to_pushover(&user_key, title, body).await;
        }
        if let Some(chat_id) = user.telegram_chat_id {
            let _ = self.send_message_to_telegram(chat_id, body).await;
        }
        Ok(())
    }

    pub async fn send_message_to_telegram(
        &self,
        chat_id: i64,
        message: &str,
    ) -> Result<(), anyhow::Error> {
        self.telegram
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("telegram bot not set"))?
            .send_message(chat_id, message)
            .await?;
        Ok(())
    }

    pub async fn send_message_to_pushover(
        &self,
        user_key: &str,
        title: Option<String>,
        body: &str,
    ) -> Result<(), anyhow::Error> {
        let pushover = self
            .pushover
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("pushover not set"))?;
        if let Some(title) = title {
            pushover
                .send_notification_with_title(user_key, &title, body)
                .await?;
        } else {
            pushover.send_notification(user_key, body).await?;
        }

        Ok(())
    }
}
