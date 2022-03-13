use teloxide::{adaptors::DefaultParseMode, prelude2::*, utils::command::BotCommand};

#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum TelegramCommand {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "notify me when there is an update")]
    NotifyMe,
}

async fn answer(
    bot: DefaultParseMode<AutoSend<Bot>>,
    message: Message,
    command: TelegramCommand,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        TelegramCommand::Help => {
            bot.send_message(message.chat.id, TelegramCommand::descriptions())
                .await?
        }
        TelegramCommand::NotifyMe => {
            bot.send_message(
                message.chat.id,
                format!(
                    "Put the following chat id on tanoshi profile settings: {}",
                    message.chat.id
                ),
            )
            .await?
        }
    };

    Ok(())
}

pub async fn run(bot: DefaultParseMode<AutoSend<Bot>>) {
    info!("start telegram bot");
    teloxide::repls2::commands_repl(bot, answer, TelegramCommand::ty()).await;
}
