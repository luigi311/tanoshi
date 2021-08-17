# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added 

- Periodic background updates
- Telegram bot notification
- Support local manga chapter as directory
- Navigate to prev/next chapter at start/end of chapter
- Loading spinner when change chapter and installing extenstion

### Changed

- Decouple db and graphql
- Replace option with result
- Check token expiration
- `get_claims` return result
- Establish database connection return result
- Serialize and Deserialize from catalogue into local_storage instead of individual fields
- Use `with_node` because `events:value` is deprecated
- Implement clippy suggestion
- Bigger fonts in input box
- Use primary color for button inside topbar
- Reduce memory consupmtion by separating compile and runtime for extension
- Reduce code duplication on `query.rs` in `tanoshi-web`

### Fixed

- cover image brightness not lowered in catalogue if favorited
- refresh all libraries
- manga title wrap

## [0.24.6] - 2021-08-03

### Fixed

- fix catalogue not fetch next page
- fix some stylings

## [0.24.5] - 2021-07-24

### Changed

- Switch from yarn to trunk
- Migrate from tailwind to sass

### Fixed

- Fix web won't load

## [0.24.4] - 2021-07-16

### Fixed

- fix bottombar showing in reader

## [0.24.3] - 2021-07-16

### Added

- show error as snackbar

### Changed

- reduce panic

## [0.24.2] - 2021-07-11

### Fixed

- fix panic when using local source

[unreleased]: https://github.com/faldez/tanoshi/compare/v0.24.6...HEAD
[0.24.6]: https://github.com/faldez/tanoshi/compare/v0.24.5...v0.24.6
[0.24.5]: https://github.com/faldez/tanoshi/compare/v0.24.4...v0.24.5
[0.24.4]: https://github.com/faldez/tanoshi/compare/v0.24.3...v0.24.4
[0.24.3]: https://github.com/faldez/tanoshi/compare/v0.24.2...v0.24.3
[0.24.2]: https://github.com/faldez/tanoshi/compare/v0.24.1...v0.24.2
