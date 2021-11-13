// from https://github.com/tauri-apps/tauri-plugin-localhost

use tauri::{
  async_runtime::block_on,
  plugin::{Plugin, Result as PluginResult},
  AppHandle, Runtime,
};

use tiny_http::{Header, Response, Server};

pub struct Proxy {
  port: u16,
  handle: tanoshi::proxy::Proxy,
}

impl Proxy {
  pub fn new(secret: &str) -> Self {
    let port = portpicker::pick_unused_port().unwrap();
    let handle = tanoshi::proxy::Proxy::new(secret.to_string());
    Self { port, handle }
  }
}

impl<R: Runtime> Plugin<R> for Proxy {
  fn name(&self) -> &'static str {
    "proxy"
  }

  fn initialization_script(&self) -> Option<String> {
    Some(format!(
      "window.__TANOSHI_IMAGE_PROXY_PORT__ = {};",
      self.port
    ))
  }

  fn initialize(&mut self, _app: &AppHandle<R>, _config: serde_json::Value) -> PluginResult<()> {
    let port = self.port;
    let handle = self.handle.clone();

    let server = Server::http(format!("127.0.0.1:{}", port)).map_err(|e| format!("{}", e))?;
    std::thread::spawn(move || {
      for request in server.incoming_requests() {
        let encrypted_url = request
          .url()
          .split("/")
          .last()
          .ok_or("no last part")
          .unwrap();
        let (content_type, data) = block_on(handle.get_image_raw(encrypted_url)).unwrap();
        let response = Response::from_data(data).with_header(Header {
          field: "Content-Type".parse().unwrap(),
          value: content_type.parse().unwrap(),
        });
        request.respond(response).unwrap();
      }
    });

    Ok(())
  }
}
