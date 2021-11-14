// from https://github.com/tauri-apps/tauri-plugin-localhost

use tauri::{
  async_runtime::block_on,
  plugin::{Plugin, Result as PluginResult},
  AppHandle, Runtime,
};

use tiny_http::{Header, Response, Server};

use tanoshi::{config::GLOBAL_CONFIG, db, local, notifier, schema, worker};
use tanoshi_vm::{bus::ExtensionBus, vm};

use std::sync::Arc;

pub struct GraphQL {
  port: u16,
}

impl GraphQL {
  pub fn new() -> Self {
    let port = portpicker::pick_unused_port().unwrap();
    Self { port }
  }
}

impl<R: Runtime> Plugin<R> for GraphQL {
  fn name(&self) -> &'static str {
    "graphql"
  }

  fn initialization_script(&self) -> Option<String> {
    Some(format!("window.__TANOSHI_GRAPHQL_PORT__ = {};", self.port))
  }

  fn initialize(&mut self, _app: &AppHandle<R>, _config: serde_json::Value) -> PluginResult<()> {
    let port = self.port;

    let (tx, rx) = std::sync::mpsc::channel();
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

      let (vm_handle, extension_tx) = vm::start();
      vm::load(&config.plugin_path, extension_tx.clone())
        .await
        .unwrap();

      let extension_bus = ExtensionBus::new(&config.plugin_path, extension_tx);
      if let Err(_) = extension_bus
        .insert(
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

      tx.send(schema).unwrap();

      tokio::select! {
          _ = vm_handle => {
            println!("vm quit");
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

    let schema = rx.recv().unwrap();

    let server = Server::http(format!("127.0.0.1:{}", port)).map_err(|e| format!("{}", e))?;
    std::thread::spawn(move || {
      for mut request in server.incoming_requests() {
        let method = request.method().to_string();
        if method == "POST" {
          let mut query_slice = vec![];
          request.as_reader().read_to_end(&mut query_slice).unwrap();

          let mut query: async_graphql::Request = serde_json::from_slice(&query_slice).unwrap();

          let token = request.headers().iter().find_map(|header| {
            if header.field.to_string().to_lowercase() == "authorization" {
              header
                .value
                .to_string()
                .strip_prefix("Bearer ")
                .map(|s| s.to_string())
            } else {
              None
            }
          });
          if let Some(token) = token {
            query = query.data(token.to_string())
          }

          let res = block_on(schema.execute(query));
          let response = Response::from_data(serde_json::to_vec(&res).unwrap())
            .with_status_code(200)
            .with_header(Header {
              field: "Content-Type".parse().unwrap(),
              value: "application/graphql".parse().unwrap(),
            })
            .with_header(Header {
              field: "Access-Control-Allow-Origin".parse().unwrap(),
              value: "*".parse().unwrap(),
            })
            .with_header(Header {
              field: "Access-Control-Allow-Methods".parse().unwrap(),
              value: "*".parse().unwrap(),
            })
            .with_header(Header {
              field: "Access-Control-Allow-Headers".parse().unwrap(),
              value: "*".parse().unwrap(),
            });
          request.respond(response).unwrap();
        } else if method == "OPTIONS" {
          let response = Response::empty(204)
            .with_header(Header {
              field: "Access-Control-Allow-Origin".parse().unwrap(),
              value: "*".parse().unwrap(),
            })
            .with_header(Header {
              field: "Access-Control-Allow-Methods".parse().unwrap(),
              value: "*".parse().unwrap(),
            })
            .with_header(Header {
              field: "Access-Control-Allow-Headers".parse().unwrap(),
              value: "*".parse().unwrap(),
            });
          request.respond(response).unwrap();
        } else {
          let response = Response::empty(404);
          request.respond(response).unwrap();
        }
      }
    });

    Ok(())
  }
}
