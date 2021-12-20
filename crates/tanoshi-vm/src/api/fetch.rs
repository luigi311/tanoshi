use std::{str::FromStr, sync::Arc};

use std::collections::HashMap;

use lazy_static::lazy_static;
use reqwest::{cookie::Jar, Method};
use rquickjs::{bind, FromJs, IntoJs};

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.150 Safari/537.36 Edg/88.0.705.63";

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::ClientBuilder::new()
        .user_agent(DEFAULT_USER_AGENT)
        .cookie_store(true)
        .cookie_provider(Arc::new(Jar::default()))
        .brotli(true)
        .deflate(true)
        .gzip(true)
        .build()
        .unwrap();
}
#[derive(Debug, Clone, FromJs, IntoJs)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, FromJs, IntoJs)]
#[quickjs(untagged)]
pub enum RequestBody {
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Default, FromJs, IntoJs)]
pub struct RequestOptions {
    method: Option<String>,
    headers: Option<HashMap<String, String>>,
    body: Option<RequestBody>,
}

#[bind(object, public)]
#[quickjs(rename = "__native_fetch__")]
pub async fn fetch(url: String, opts: Option<RequestOptions>) -> rquickjs::Result<Response> {
    let opts = opts.unwrap_or_default();

    let method = opts
        .method
        .and_then(|method| Method::from_str(&method).ok())
        .unwrap_or(Method::GET);

    let mut req = CLIENT.request(method, &url);
    for (name, value) in opts.headers.unwrap_or_default() {
        req = req.header(&name, &value);
    }

    if let Some(body) = opts.body {
        req = match body {
            RequestBody::String(text) => req.body(text),
            RequestBody::Bytes(bytes) => req.body(bytes),
        }
    }

    let res = req
        .send()
        .await
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;

    let status = res.status().as_u16();

    let mut headers = HashMap::new();
    for (name, value) in res.headers() {
        headers.insert(
            name.to_string(),
            value
                .to_str()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?
                .to_string(),
        );
    }

    let body = res
        .bytes()
        .await
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?
        .to_vec();

    Ok(Response {
        status,
        headers,
        body,
    })
}

#[bind(object, public)]
pub fn bytes_to_string(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_default()
}
