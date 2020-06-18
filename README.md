![Build](https://github.com/faldez/tanoshi/workflows/Build/badge.svg)

# <img src="tanoshi-web/static/apple-touch-icon.png" alt="" width="30" height=30/> Tanoshi
Selfhosted Tachiyomi-like web manga reader.

## Features
### Currently working
- Browse, search, and read manga from local, mangadex and [more](https://github.com/fadhlika/tanoshi-extensions)
- Favorite mangas
- Reading history across devices
- See chapter updates
- Read in single page, double page, or long strip
- Read from right to left or left to right
- Web reader is PWA and optimized for standalone mode on mobile

### Planned
My plan is to make this as close as tachiyomi features. Planned features are listed [here](https://github.com/faldez/tanoshi/issues?q=is%3Aopen+is%3Aissue+label%3Aenhancement)

## Why Rust
Rust is the most loved programming language, I thought this is my chance to learn rust too.

## Installation
### Prebuilt Binary
Download and run binary from latest release, aside from plugins all dependencies are statically linked. It should run find on any linux x64 machine.

### Plugins
To browse and read manga, tanoshi needs plugins. Plugins can be donwnloaded from [here](https://github.com/fadhlika/tanoshi-extensions) in `repo` branch. 
You can download individual plugins and store in on your selected `plugins_path` in `config.yml`, or clone `repo` branch and copy all of plugins to `plugins_path`.

Or you can install from web and restart tanoshi afterward.

## Usage
### CLI
```
tanoshi 

USAGE:
    tanoshi [FLAGS] [OPTIONS]

FLAGS:
        --create-admin    Create initial admin user account
    -h, --help            Prints help information
    -V, --version         Prints version information

OPTIONS:
        --config <config>    Path to config file [default: ~/config/tanoshi/config.yml]
```

### Config
Tanoshi default to look configuration in `~/.config/tanoshi/config.yml`. Below is example configuration
```
# Port for tanoshi to server, default to 80
port: 3030
# URL to database
database_url: postgres://username:password@127.0.0.1:5432/tanoshi
# JWT secret, any random value, changing this will render any active token invalid
secret: secret
# Cache time to live, period in days for cache purge
cache_ttl: 1
# Where plugin is stored
plugin_path: ~/.tanoshi/plugins
#This section is for plugin configuration
plugin_config:
  # Plugin name
  local:
    path: ~/mangas
```

## Demo Video
### Mobile
![mobile](screenshots/mobile.gif)

### Desktop
![desktop](screenshots/desktop.gif)
