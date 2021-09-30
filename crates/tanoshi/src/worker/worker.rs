use teloxide::{
    adaptors::{AutoSend, DefaultParseMode},
    prelude::Requester,
    Bot,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::{sync::mpsc::unbounded_channel, task::JoinHandle};

use crate::notifier::pushover::Pushover;

pub enum Command {
    TelegramMessage(i64, String),
    PushoverMessage {
        user_key: String,
        title: Option<String>,
        body: String,
    },
}

struct Worker {
    telegram_bot: Option<DefaultParseMode<AutoSend<Bot>>>,
    pushover: Option<Pushover>,
}

impl Worker {
    fn new(
        telegram_bot: Option<DefaultParseMode<AutoSend<Bot>>>,
        pushover: Option<Pushover>,
    ) -> Self {
        Self {
            telegram_bot,
            pushover,
        }
    }

    async fn run(&self, rx: UnboundedReceiver<Command>) {
        let mut rx = rx;

        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::TelegramMessage(chat_id, message) => {
                    if let Some(bot) = self.telegram_bot.as_ref() {
                        if let Err(e) = bot.send_message(chat_id, message).await {
                            error!("failed to send TelegramMessage, reason: {}", e);
                        }
                    }
                }
                Command::PushoverMessage {
                    user_key,
                    title,
                    body,
                } => {
                    if let Some(pushover) = self.pushover.as_ref() {
                        if let Some(title) = title {
                            if let Err(e) = pushover
                                .send_notification_with_title(&user_key, &title, &body)
                                .await
                            {
                                error!("failed to send PushoverMessage, reason: {}", e);
                            }
                        } else  if let Err(e) = pushover.send_notification(&user_key, &body).await {
                            error!("failed to send PushoverMessage, reason: {}", e);
                        }
                    }
                }
            }
        }
    }
}

pub fn start(
    telegram_bot: Option<DefaultParseMode<AutoSend<Bot>>>,
    pushover: Option<Pushover>,
) -> (JoinHandle<()>, UnboundedSender<Command>) {
    let (tx, rx) = unbounded_channel();
    let worker = Worker::new(telegram_bot, pushover);

    let handle = tokio::spawn(async move {
        worker.run(rx).await;
    });

    (handle, tx)
}
