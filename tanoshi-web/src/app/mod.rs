pub mod app;
pub use app::{App, AppRoute};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub mod api;
pub mod browse;
pub mod catalogue;
pub mod component;
pub mod detail;
pub mod history;
pub mod home;
pub mod job;
pub mod login;
pub mod logout;
pub mod reader;
pub mod select;
pub mod settings;
pub mod source;
