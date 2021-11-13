#[macro_use]
extern crate log;
extern crate argon2;

#[cfg(feature = "embed")]
pub mod assets;
pub mod catalogue;
pub mod config;
pub mod db;
pub mod downloads;
pub mod library;
pub mod local;
pub mod notifier;
pub mod proxy;
pub mod schema;
#[cfg(feature = "server")]
pub mod server;
pub mod status;
pub mod user;
pub mod utils;
pub mod worker;
