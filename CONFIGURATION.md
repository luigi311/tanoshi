# Configuration

Tanoshi reads its server settings from a YAML file named `config.yml`. The server
and desktop app use the same configuration format. Edit the file and restart
Tanoshi to apply changes.

## Configuration file and data directory

By default, `config.yml` is stored in Tanoshi's data directory. Tanoshi chooses
that directory in this order:

1. The directory specified by the `TANOSHI_HOME` environment variable.
2. `~/.tanoshi`, if that legacy directory already exists.
3. The platform's application data directory:

   | Platform | Default data directory |
   | -------- | ---------------------- |
   | Linux | `$XDG_DATA_HOME/tanoshi`, or `~/.local/share/tanoshi` when `XDG_DATA_HOME` is unset |
   | macOS | `~/Library/Application Support/org.luigi311.tanoshi` |
   | Windows | `%APPDATA%\luigi311\tanoshi\data` |

The Docker image sets `TANOSHI_HOME=/tanoshi`, so its configuration file is
`/tanoshi/config.yml`. The repository's Compose example mounts `./tanoshi` there.

For the standalone server, select a different configuration file with:

```sh
tanoshi --config /path/to/config.yml
```

If the file does not exist, Tanoshi writes a default configuration, including a
generated authentication secret. Create the parent directory first when using a
custom configuration location. Choosing a different configuration file does not
change the default data directory or storage paths.

In the tables below, `<data_dir>` means the directory selected above. Paths in
YAML are literal: `~` and environment variables are not expanded, and relative
paths are resolved from the process's working directory, not from the location
of `config.yml`.

## Full configuration example

This example uses explicit paths for a server installation. Adjust the paths
and public URL for your system, create the storage directories, and replace the
example secret with your own random value or keep the secret from the generated
configuration. The optional integrations are disabled here; their sections below
show how to enable them.

```yaml
base_url: "https://manga.example.com"
port: 8080
secret: "replace-with-your-own-random-secret"
enable_playground: false

database_path: "/srv/tanoshi/tanoshi.db"
create_database: true
plugin_path: "/srv/tanoshi/plugins"
local_path: "/srv/tanoshi/manga"
download_path: "/srv/tanoshi/downloads"
cache_path: "/srv/tanoshi/cache"

update_interval: 3600
max_concurrent_update_sources: 2
auto_download_chapters: false

extension:
  max_concurrent_calls_per_source: 8
  metadata_timeout_secs: 30
  image_timeout_secs: 120

telegram: null
pushover: null
gotify: null
myanimelist: null
anilist: null
```

Top-level settings may be omitted to use their defaults. Optional integration
objects default to `null` (disabled). When an integration object is supplied,
all of its fields listed below are required.

## Server settings

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `base_url` | `null` | Public URL used for chapter links in notifications and tracker sign-in callbacks. Include the scheme and omit the trailing slash, for example `https://manga.example.com`. Required when either tracker is configured. |
| `port` | `80` | HTTP listening port for the standalone server, which listens on all IPv4 interfaces (`0.0.0.0`). The desktop app instead selects an available port on `127.0.0.1`. |
| `secret` | Generated 16-character alphanumeric string | Secret used to sign authentication tokens. Keep the generated value or supply your own stable random value; changing it invalidates existing sign-in tokens. |
| `enable_playground` | `false` | Enable the browser-based GraphQL Playground at `/graphql`. |

Keep `secret` in an existing configuration file. If it is omitted, a new value
is generated each time that file is loaded.

## Storage settings

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `database_path` | `<data_dir>/tanoshi.db` | Path to the SQLite database file. |
| `create_database` | `true` | Create the database if it does not exist. Set to `false` to require an existing database. Database migrations still run at startup. |
| `plugin_path` | `<data_dir>/plugins` | Directory containing installed source extensions. |
| `local_path` | `<data_dir>/manga` | Local manga directory, or a list of named directories as described below. |
| `download_path` | `<data_dir>/downloads` | Directory for downloaded chapters and their manga metadata. |
| `cache_path` | `<data_dir>/cache` | Directory for cached images and update information. |

### Local folders

`local_path` accepts either a single directory string or a list of objects. A
single directory appears as a source named **Local**:

```yaml
local_path: "/srv/manga"
```

To expose several folders as separate sources, give each one a `name` and `path`:

```yaml
local_path:
  - name: "Manga"
    path: "/srv/manga"
  - name: "Comics"
    path: "/srv/comics"
```

| Field | Type | Description |
| ----- | ---- | ----------- |
| `name` | String | Required display name for this local source. |
| `path` | String | Required path to its manga directory. |

Local sources support image folders and `.cbz`, `.cbr`, and `.cb7` comic archives.
Use `local_path: []` to register no local sources. Keep existing list entries in
the same order after adding local manga to your library: local source IDs are
assigned from their positions in this list.

## Chapter updates and downloads

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `update_interval` | `3600` | Seconds between automatic chapter update checks. Release builds use a minimum of `3600` seconds (one hour). `0` also uses `3600`; it does not disable updates. Debug builds allow shorter positive intervals. |
| `max_concurrent_update_sources` | `2` | Maximum number of sources checked for chapter updates at the same time. `0` is treated as `1`. This is separate from the number of requests allowed within each source. |
| `auto_download_chapters` | `false` | Automatically queue newly discovered chapters for manga in at least one user's library, skipping chapters already downloaded. Manual downloads are available regardless of this setting. |

## Notifications

Configure any combination of the following objects to enable notification
providers. Each user then supplies their notification destination in their
profile. Omitting a provider object or setting it to `null` disables that provider.

### Telegram (`telegram`)

```yaml
telegram:
  name: "my_tanoshi_bot"
  token: "your-telegram-bot-token"
```

| Field | Type | Description |
| ----- | ---- | ----------- |
| `name` | String | Bot name. Required by the configuration format; notification delivery uses `token`. |
| `token` | String | API token for the Telegram bot. |

Send `/notifyme` to the bot to obtain your chat ID, then save that ID in your
Tanoshi profile's Telegram field.

### Pushover (`pushover`)

```yaml
pushover:
  application_key: "your-pushover-application-key"
```

| Field | Type | Description |
| ----- | ---- | ----------- |
| `application_key` | String | Pushover application/API token used by the server. |

Each user saves their own Pushover user key in their Tanoshi profile.

### Gotify (`gotify`)

```yaml
gotify:
  base_url: "https://gotify.example.com"
```

| Field | Type | Description |
| ----- | ---- | ----------- |
| `base_url` | String | URL of the Gotify server, without a trailing slash or `/message` suffix. This is separate from Tanoshi's top-level `base_url`. |

Each user saves their Gotify application token in their Tanoshi profile.

## Manga tracking

Both tracker integrations require the top-level `base_url`; Tanoshi cannot start
with a tracker configured and `base_url` missing. Use the public address through
which users access Tanoshi and register the matching callback URL with each
tracker application. Each user then signs in to the tracker from their profile.

### MyAnimeList (`myanimelist`)

```yaml
base_url: "https://manga.example.com"
myanimelist:
  client_id: "your-myanimelist-client-id"
  client_secret: "your-myanimelist-client-secret"
```

| Field | Type | Description |
| ----- | ---- | ----------- |
| `client_id` | String | Client ID of your MyAnimeList OAuth application. |
| `client_secret` | String | Client secret of that application. |

For this example, the callback URL is
`https://manga.example.com/tracker/myanimelist/redirect`.

### AniList (`anilist`)

```yaml
base_url: "https://manga.example.com"
anilist:
  client_id: "your-anilist-client-id"
  client_secret: "your-anilist-client-secret"
```

| Field | Type | Description |
| ----- | ---- | ----------- |
| `client_id` | String | Client ID of your AniList OAuth application. Quote numeric IDs so YAML reads them as strings. |
| `client_secret` | String | Client secret of that application. |

For this example, the callback URL is
`https://manga.example.com/tracker/anilist/redirect`.

## Extension concurrency

The `extension` object is optional. Its fields also have individual defaults, so
you can set only the limits you want to change.

| Field | Default | Description |
| ----- | ------- | ----------- |
| `max_concurrent_calls_per_source` | `8` | Maximum simultaneous calls for each source, shared by users and background jobs. `0` is treated as `1`. |
| `metadata_timeout_secs` | `30` | Timeout in seconds for metadata requests and source preference changes after admission. |
| `image_timeout_secs` | `120` | Timeout in seconds for image requests after admission. |

Use positive timeout values. A value of `0` is clamped to one millisecond; it
does not disable the timeout. The former `extension.admission_timeout_ms` setting
has been removed.

Installed sources share one extension process per source. Read requests from all
users and background jobs can run concurrently, with a default limit of 8 calls
per source. Each extension's own rate limiter still controls request pacing.

Each source has an independent queue in the VM. User activity, including manual
refreshes, has high priority; downloads and scheduled updates have low priority.
When a slot becomes available, queued high-priority requests run first. Requests
with the same priority run in arrival order. Running calls finish normally.

Requests wait for a slot without an admission timeout. Metadata and image
deadlines start after a slot is acquired and include time spent waiting in the
extension's rate limiter. After admission, preference changes wait in the host
until running reads finish and hold back new reads while the update completes.
That wait uses the metadata deadline; a timeout while waiting for reads to finish
does not retire the worker or count as a source failure.

Configure the limits in `config.yml`:

```yaml
extension:
  max_concurrent_calls_per_source: 8
  metadata_timeout_secs: 30
  image_timeout_secs: 120
```

The global `max_concurrent_calls_per_source` setting currently applies the same
limit to every source independently. The planned design will remove this global
concurrency setting and dynamically determine each source's concurrency from the
rate limits defined by its extension.

An executing call that times out retires its worker. Other running calls get up
to one second to finish before the process is terminated. Reads and preference
replacements interrupted by timeout recovery retry within their original
deadlines. Calls interrupted by an unattributed crash get at most one automatic
crash retry per request, even if successful peers reset the source's health
counter. Calls still queued in the host can retry without consuming that crash
allowance. A replacement starts after the old process exits and replays its
acknowledged preferences. Retries check current source health before replacement
and dispatch; a quarantined source rejects further work.

A request that expires before writing any bytes returns a queue timeout and
leaves the worker running. Expired writer entries are removed before enqueueing
new work, and a full queue reports backpressure rather than a worker crash.
A timeout during a partial frame terminates the process immediately because the
request stream cannot safely continue. A fully written frame keeps the normal
drain period even if flush misses its deadline. Timeouts do not quarantine a
source. An unattributed crash with pending calls counts as one health failure;
idle worker exits do not count. Three consecutive health failures without a
successful call quarantine the source until it is reloaded. Successful calls
reset that health counter, but never a request's crash-retry allowance.

## Environment variables and logging

Set these in the environment used to start Tanoshi; they are not YAML keys.

| Variable | Description |
| -------- | ----------- |
| `TANOSHI_HOME` | Select the data directory used for `config.yml` and default storage paths. |
| `RUST_LOG` | Logging filter, such as `info` or `tanoshi=debug,tanoshi_vm=debug`. Takes precedence over `TANOSHI_LOG` in the standalone server. |
| `TANOSHI_LOG` | Standalone server shorthand for setting the `tanoshi` and `tanoshi_vm` log levels, for example `info` or `debug`. The Docker image defaults this to `info`. The desktop app uses `RUST_LOG` directly. |
| `TANOSHI_EXTENSION_WORKER` | Advanced override for the extension worker executable. The server and desktop app normally use their own executable automatically; a replacement must be compatible with the running Tanoshi version. |

For example, start the server with normal application logs and detailed download
queue logs:

```sh
RUST_LOG=tanoshi=info,tanoshi_vm=info,tanoshi::download_queue=debug tanoshi
```

The listening port is configured by `port` in `config.yml`, not by the Docker
image's `PORT` environment variable. Other YAML settings do not have automatic
environment-variable overrides.

## Automatically managed settings

`extension_repository` is selected automatically for the Rust version used to
build Tanoshi. It is not read from or written to `config.yml`, so adding that key
does not override the repository. The configuration file's own `path` is also
managed internally rather than set through YAML.

The supported YAML settings and defaults are defined in
[`config.rs`](crates/tanoshi/src/infrastructure/config.rs).
