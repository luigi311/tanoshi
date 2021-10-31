pub mod downloads;
pub mod updates;

use teloxide::{
    adaptors::{AutoSend, DefaultParseMode},
    prelude::Requester,
    Bot,
};
use tokio::sync::mpsc::{Receiver, Sender, UnboundedSender};
use tokio::task::JoinHandle;

use crate::notifier::pushover::Pushover;

pub enum Command {
    TelegramMessage(i64, String),
    PushoverMessage {
        user_key: String,
        title: Option<String>,
        body: String,
    },
    StartDownload,
}

struct Worker {
    telegram_bot: Option<DefaultParseMode<AutoSend<Bot>>>,
    pushover: Option<Pushover>,
    download_tx: UnboundedSender<()>,
    rx: Receiver<Command>,
}

impl Worker {
    fn new(
        telegram_bot: Option<DefaultParseMode<AutoSend<Bot>>>,
        pushover: Option<Pushover>,
        download_tx: UnboundedSender<()>,
        rx: Receiver<Command>,
    ) -> Self {
        Self {
            telegram_bot,
            pushover,
            download_tx,
            rx,
        }
    }

    async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
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
                        } else if let Err(e) = pushover.send_notification(&user_key, &body).await {
                            error!("failed to send PushoverMessage, reason: {}", e);
                        }
                    }
                }
                Command::StartDownload => {
                    if let Err(e) = self.download_tx.send(()) {
                        error!("failed send download command: {}", e);
                    }
                }
            }
        }
    }
}

pub fn start(
    telegram_bot: Option<DefaultParseMode<AutoSend<Bot>>>,
    pushover: Option<Pushover>,
    download_tx: UnboundedSender<()>,
) -> (JoinHandle<()>, Sender<Command>) {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let mut worker = Worker::new(telegram_bot, pushover, download_tx, rx);

    let handle = tokio::spawn(async move {
        worker.run().await;
    });

    (handle, tx)
}
