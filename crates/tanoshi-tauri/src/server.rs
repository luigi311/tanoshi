use log::{error, info, warn};
use tauri::{plugin::Plugin, AppHandle, Runtime};

use tanoshi::{application::bootstrap, infrastructure::config::Config};

pub struct Server {
  port: u16,
}

impl Server {
  pub fn new() -> Self {
    let port = portpicker::pick_unused_port().expect("no free port for the embedded server");
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

  fn initialize(&mut self, _app: &AppHandle<R>, _config: serde_json::Value) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let port = self.port;

    tauri::async_runtime::spawn(async move {
      let config = match Config::open::<String>(None) {
        Ok(config) => config,
        Err(e) => {
          error!("failed to open config: {e}");
          return;
        }
      };

      let app = match bootstrap::bootstrap(config).await {
        Ok(app) => app,
        Err(e) => {
          error!("failed to start embedded server: {e}");
          return;
        }
      };

      // The desktop app has no ctrl-c of its own; the bot loop and workers
      // end when the webview process exits.
      if let Some(telegram_bot) = app.telegram_bot {
        tauri::async_runtime::spawn(telegram_bot);
      }

      let server_fut = app.server_builder.serve(([127, 0, 0, 1], port));

      info!("server listening on port {port}");

      tokio::select! {
          _ = server_fut => {
              info!("server shutdown");
          }
          _ = app.update_worker_handle => {
              warn!("update worker quit unexpectedly");
          }
          _ = app.download_worker_handle => {
              warn!("download worker quit unexpectedly");
          }
          _ = tokio::signal::ctrl_c() => {
              info!("ctrl+c signal received, shutting down");
          }
      }

      app.pool.close().await;
    });

    Ok(())
  }
}
