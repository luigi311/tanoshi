use tbot::contexts::fields::Message;
use tbot::{markup::markdown_v2, prelude::*, types::parameters::Text, util::entities, Bot as TBot};

pub struct Bot {
    bot: TBot,
}

impl Bot {
    pub fn new(token: String) -> Self {
        let bot = TBot::new(token);
        Bot { bot }
    }

    pub fn start(&self) {
        let bot = self.bot.clone();
        tokio::spawn(async move {
            let mut bot = bot.event_loop();

            bot.start(|context| async move {
                let chat_id = context.clone().chat().id.0;
                context.send_message(
                    Text::plain(
                        format!("This chat id is {}. Input this chat id in setting to get notification on chapter updates", chat_id)
                            .as_str()
                    ))
                    .call()
                    .await
                    .unwrap();
            });

            bot.polling().start().await.unwrap();
        });
    }
}
