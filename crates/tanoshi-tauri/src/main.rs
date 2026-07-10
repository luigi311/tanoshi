#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

extern crate log;
use std::env;
use tauri::Manager;

mod server;

use crate::server::Server;

fn main() {
  env_logger::init();

  tauri::Builder::default()
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_clipboard_manager::init())
    .plugin(tauri_plugin_fs::init())
    .plugin(Server::new())
    .setup(|app| {
      let window = app
        .get_webview_window("main")
        .expect("main webview window missing");

      // Check if running on Phosh
      if let Ok(session) = env::var("XDG_SESSION_DESKTOP") {
        println!("Session: {session}");
        // Hide title bar if running on Phosh
        if let Err(e) = window.set_decorations(session != "phosh") {
          eprintln!("failed to set window decorations: {e}");
        }
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
