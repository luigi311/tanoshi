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

### Planned
My plan is to make this as close as tachiyomi features. Planned features are listed [here](https://github.com/faldez/tanoshi/issues?q=is%3Aopen+is%3Aissue+label%3Aenhancement)

## Why Rust
Rust is the most loved programming language, I thought this is my chance to learn rust too.

## Installation
### Database
Tanoshi use postgresql for database, simply create user, database, and run migration script from `tanoshi/migration/tanoshi.sql` and you are good to go.

### Prebuilt Binary
Download and run binary from latest release, aside from plugins all dependencies are statically linked.

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
# Where plugin is stored
plugin_path: ~/.tanoshi/plugins
#This section is for plugin configuration
plugin_config:
  # Plugin name
  local:
    path: /Users/fadhlika/Repos/tanoshi/mangas
```

## Demo Video
### Mobile
![Imgur](https://imgur.com/fzrIlP0)'

### Desktop
![Imgur](https://imgur.com/7M7FlTc)