#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

extern crate log;
use std::{env, ffi::OsString, path::PathBuf};
use tauri::Manager;

mod server;

use crate::server::Server;

fn run_as_extension_worker() -> bool {
  let mut args = env::args_os();
  let _program = args.next();
  if args.next() != Some(OsString::from("--tanoshi-extension-worker")) {
    return false;
  }

  let plugin_path = match (args.next(), args.next(), args.next()) {
    (Some(flag), Some(path), None) if flag == OsString::from("--plugin") => PathBuf::from(path),
    _ => {
      eprintln!("usage: tanoshi-app --tanoshi-extension-worker --plugin <path>");
      std::process::exit(2);
    }
  };

  if let Err(error) = tanoshi_vm::extension::worker::run_worker(plugin_path) {
    eprintln!("extension worker failed: {error:#}");
    std::process::exit(1);
  }
  true
}

fn main() {
  if run_as_extension_worker() {
    return;
  }

  if env::var_os("TANOSHI_EXTENSION_WORKER").is_none() {
    if let Ok(executable) = env::current_exe() {
      // Set this before Tauri starts any threads so bundled applications can
      // supervise workers with their own executable.
      unsafe { env::set_var("TANOSHI_EXTENSION_WORKER", executable) };
    }
  }

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
