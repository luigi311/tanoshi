#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use tanoshi::config::{Config, GLOBAL_CONFIG};

use crate::server::Server;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod server;

fn main() {
  GLOBAL_CONFIG.get_or_init(|| Config::open::<String>(None).expect("failed to init config"));

  tauri::Builder::default()
    .plugin(Server::new())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
