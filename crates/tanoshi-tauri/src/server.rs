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

      let mut update_worker_handle = app.update_worker_handle;
      let mut download_worker_handle = app.download_worker_handle;
      let telegram_bot_handle = app.telegram_bot.map(tokio::spawn);

      let server_fut = app.server_builder.serve(([127, 0, 0, 1], port));

      info!("server listening on port {port}");

      tokio::select! {
          _ = server_fut => {
              info!("server shutdown");
          }
          _ = &mut update_worker_handle => {
              warn!("update worker quit unexpectedly");
          }
          _ = &mut download_worker_handle => {
              warn!("download worker quit unexpectedly");
          }
          _ = tokio::signal::ctrl_c() => {
              info!("ctrl+c signal received, shutting down");
          }
      }

      // stop the remaining tasks before closing the pool they hold clones of;
      // a finished handle is skipped since its output may already be consumed
      for handle in [update_worker_handle, download_worker_handle]
        .into_iter()
        .chain(telegram_bot_handle)
      {
        if !handle.is_finished() {
          handle.abort();
          let _ = handle.await;
        }
      }

      app.pool.close().await;
    });

    Ok(())
  }
}
