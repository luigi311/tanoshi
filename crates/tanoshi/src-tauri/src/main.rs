#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use tanoshi::config::{Config, GLOBAL_CONFIG};

use proxy::Proxy;

use crate::graphql::GraphQL;

extern crate tiny_http;

mod graphql;
mod proxy;

fn main() {
  let config =
    GLOBAL_CONFIG.get_or_init(|| Config::open::<String>(None).expect("failed to init config"));

  tauri::Builder::default()
    .plugin(GraphQL::new())
    .plugin(Proxy::new(&config.secret))
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
