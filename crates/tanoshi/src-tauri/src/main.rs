#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

#[cfg(not(target_os = "macos"))]
fn main() {
  use tauri::{async_runtime::block_on, http::ResponseBuilder, Manager};

  use tanoshi::{
    config::{Config, GLOBAL_CONFIG},
    db, local,
    proxy::Proxy,
    schema::{self, TanoshiSchema},
    worker,
  };
  use tanoshi_vm::{bus::ExtensionBus, vm};

  use std::sync::Arc;

  let config =
    GLOBAL_CONFIG.get_or_init(|| Config::open::<String>(None).expect("failed to init config"));

  tauri::Builder::default()
    .setup(|app| {
      let window = app.get_window("main").unwrap();
      let (tx, rx) = std::sync::mpsc::channel();
      tauri::async_runtime::spawn(async move {
        let config = GLOBAL_CONFIG.get().unwrap();

        let pool = match db::establish_connection(&config.database_path).await {
          Ok(pool) => pool,
          Err(err) => {
            window
              .emit("message", Some(format!("'{}'", err)))
              .expect("failed to emit event");
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
        if let Err(err) = extension_bus
          .insert(
            local::ID,
            Arc::new(local::Local::new(config.local_path.clone())),
          )
          .await
        {
          window
            .emit("message", Some(format!("'{}'", err)))
            .expect("failed to emit event");
          return;
        }

        let (download_tx, download_worker_handle) = worker::downloads::start(
          &config.download_path,
          mangadb.clone(),
          extension_bus.clone(),
          None,
          None,
        );

        let update_worker_handle = worker::updates::start(
          config.update_interval,
          userdb.clone(),
          mangadb.clone(),
          extension_bus.clone(),
          download_tx.clone(),
          None,
          None,
        );

        let schema = schema::build(userdb, mangadb, extension_bus, download_tx, None, None);

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
      app.manage(schema);

      Ok(())
    })
    .register_uri_scheme_protocol("graphql", move |app, req| {
      if req.method() == "POST" {
        let schema = app.state::<TanoshiSchema>();

        let mut query: async_graphql::Request = serde_json::from_slice(&req.body)?;

        if let Some(token) = req
          .headers()
          .get("Authorization")
          .and_then(|val| val.to_str().ok())
          .and_then(|val| val.strip_prefix("Bearer "))
        {
          query = query.data(token.to_string())
        }

        let res = block_on(schema.execute(query));
        ResponseBuilder::new()
          .mimetype("application/graphql")
          .header("Access-Control-Allow-Origin", "*")
          .header("Access-Control-Allow-Methods", "*")
          .header("Access-Control-Allow-Headers", "*")
          .status(200)
          .body(serde_json::to_vec(&res)?)
      } else if req.method() == "OPTIONS" {
        ResponseBuilder::new()
          .header("Access-Control-Allow-Origin", "*")
          .header("Access-Control-Allow-Methods", "*")
          .header("Access-Control-Allow-Headers", "*")
          .status(204)
          .body(vec![])
      } else {
        ResponseBuilder::new().status(404).body(vec![])
      }
    })
    .register_uri_scheme_protocol("images", move |app, req| {
      let proxy = app.try_state::<Arc<Proxy>>().ok_or("no proxy set")?;

      let encrypted_url = req.uri().split("/").last().ok_or("no last part")?;
      let (content_type, data) = block_on(proxy.get_image_raw(encrypted_url))?;
      ResponseBuilder::new()
        .status(200)
        .header("Content-Type", content_type)
        .body(data)
    })
    .manage(Proxy::new(config.secret.clone()))
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[cfg(target_os = "macos")]
fn main() {}
