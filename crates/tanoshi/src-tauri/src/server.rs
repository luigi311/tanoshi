// from https://github.com/tauri-apps/tauri-plugin-localhost

use tauri::{
  plugin::{Plugin, Result as PluginResult},
  AppHandle, Runtime,
};

use tanoshi::{config::GLOBAL_CONFIG, db, local, notifier, schema, server, worker};
use tanoshi_vm::{bus::ExtensionBus, vm};

use std::sync::Arc;

pub struct Server {
  port: u16,
}

impl Server {
  pub fn new() -> Self {
    let port = portpicker::pick_unused_port().unwrap();
    Self { port }
  }
}

impl<R: Runtime> Plugin<R> for Server {
  fn name(&self) -> &'static str {
    "graphql"
  }

  fn initialization_script(&self) -> Option<String> {
    Some(format!("window.__TANOSHI_PORT__ = {};", self.port))
  }

  fn initialize(&mut self, _app: &AppHandle<R>, _config: serde_json::Value) -> PluginResult<()> {
    let port = self.port;

    tauri::async_runtime::spawn(async move {
      let config = GLOBAL_CONFIG.get().unwrap();

      let pool = match db::establish_connection(&config.database_path).await {
        Ok(pool) => pool,
        Err(_) => {
          return;
        }
      };

      let mangadb = db::MangaDatabase::new(pool.clone());
      let userdb = db::UserDatabase::new(pool.clone());

      let (_, extension_tx) = vm::start();
      vm::load(&config.plugin_path, extension_tx.clone()).unwrap();

      let extension_bus = ExtensionBus::new(&config.plugin_path, extension_tx);
      if let Err(_) = extension_bus
        .insert_async(
          local::ID,
          Arc::new(local::Local::new(config.local_path.clone())),
        )
        .await
      {
        return;
      }

      let notifier = notifier::Builder::new(userdb.clone()).finish();

      let (download_tx, download_worker_handle) = worker::downloads::start(
        &config.download_path,
        mangadb.clone(),
        extension_bus.clone(),
        notifier.clone(),
      );

      let update_worker_handle = worker::updates::start(
        config.update_interval,
        mangadb.clone(),
        extension_bus.clone(),
        download_tx.clone(),
        notifier.clone(),
      );

      let schema = schema::build(userdb, mangadb, extension_bus, download_tx, notifier);

      let app = server::init_app(&config, schema);
      let server_fut = server::serve("127.0.0.1", port, app);

      tokio::select! {
          _ = server_fut => {
              println!("server shutdown");
          }
          _ = update_worker_handle => {
              println!("update worker quit");
          }
          _ = download_worker_handle => {
              println!("download worker quit");
          }
          _ = tokio::signal::ctrl_c() => {
              println!("ctrl+c signal");
          }
      }

      pool.close().await;
    });

    Ok(())
  }
}
