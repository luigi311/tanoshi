# Notifications

## Telegram


### Create a Telegram Bot

Telegram notification is delivered via Telegram Bot, so you need to create a bot. You can look how to create a bot [here](https://core.telegram.org/bots#6-botfather).

### Config

After your bot is created, put below config to your `config.yml`.

```yaml
telegram:
  name: <your bot name>
  token: <your bot token>
```

then replace `<your bot name>` to the bot's name and `<your bot token>` with token given by [@BotFather](https://t.me/botfather).

### Chat ID

Open Telegram, go to bot chats then issue `/notifyme` command. The bot will reply with your chat id, copy the number, open Tanoshi > More > your username > Notification > Telegram chat id. You can click test to verify the bot is working, then click submit to register the chat id.


## Pushover

TODO