# Configuration File

Tanoshi will look `config.yml` in `$TANOSHI_HOME` which defaults to `$HOME/.tanoshi` on macos and linux, `C:\Users\<username>\.tanoshi` on windows. If config file doesn't exists, tanoshi will generate new file. Below is example configuration

## Example

```yaml
# Port for tanoshi to server, default to 80, ignored in desktop version
port: 3030
# Absolute path to database
database_path: /absolute/path/to/database
# JWT secret, any random value, changing this will render any active token invalid
secret: <16 alphanumeric characters>
# Absolute path to where plugin is stored
plugin_path: /absolute/path/to/plugins
# Absolute path to local manga
local_path: /absolute/path/to/manga
# Absolute path to downloaded manga
download_path: /absolute/path/to/manga
# Periodic update interval in seconds, must be over 3600, set to 0 to disable
update_interval: 3600
# Automatically download chapter on update,set to true to enable
auto_download_chapters: false
# GraphQL playground, set to true to enable
enable_playground: false
# Telegram token
telegram:
  name: <your bot name>
  token: <your bot token>
# Pushover
pushover:
  application_key: <your pushover application key>
```