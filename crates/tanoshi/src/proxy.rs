use std::sync::Arc;

#[cfg(feature = "server")]
use axum::{
    body::Body,
    extract::{Extension, Path},
    http::{Response, StatusCode},
    response::IntoResponse,
};

use crate::utils;

pub struct Proxy {
    client: reqwest::Client,
    secret: String,
}

impl Proxy {
    pub fn new(secret: String) -> Arc<Self> {
        Arc::new(Self {
            client: reqwest::Client::new(),
            secret,
        })
    }

    #[cfg(feature = "server")]
    pub async fn proxy(Path(url): Path<String>, state: Extension<Arc<Self>>) -> impl IntoResponse {
        debug!("encrypted image url: {}", url);
        let url = match utils::decrypt_url(&state.secret, &url) {
            Ok(url) => url,
            Err(e) => {
                error!("error validate url: {}", e);
                "".to_string()
            }
        };
        debug!("get image from {}", url);
        let res: Response<Body> = match state.as_ref().get_image(&url).await {
            Ok(body) => body,
            Err(status) => http::Response::builder()
                .status(status)
                .body(Body::empty())
                .unwrap(),
        };

        res
    }

    #[cfg(feature = "server")]
    pub async fn get_image(&self, url: &str) -> Result<http::Response<Body>, StatusCode> {
        match url {
            url if url.starts_with("http") => Ok(self.get_image_from_url_stream(url).await?),
            url if !url.is_empty() => {
                let (content_type, data) = self
                    .get_image_from_file(url)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                Ok(http::Response::builder()
                    .header("Content-Type", content_type)
                    .body(Body::from(data))
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
            }
            _ => Err(StatusCode::BAD_REQUEST),
        }
    }

    async fn get_image_from_file(&self, file: &str) -> Result<(String, Vec<u8>), anyhow::Error> {
        let file = std::path::PathBuf::from(file);

        // if file is already a file, serve it
        if file.is_file() {
            let content_type = mime_guess::from_path(&file)
                .first_or_octet_stream()
                .to_string();
            let data = tokio::fs::read(file).await?;
            Ok((content_type, data))
        } else {
            // else if its combination of archive files and path inside the archive
            // extract the file from archive
            let filename = file
                .parent()
                .ok_or(anyhow::anyhow!("no parent"))?
                .display()
                .to_string();
            let path = file
                .file_name()
                .ok_or(anyhow::anyhow!("no filename"))?
                .to_str()
                .ok_or(anyhow::anyhow!("failed convert to string"))?
                .to_string();
            let content_type = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            let filename = filename.clone();
            let path = path.clone();

            tokio::task::spawn_blocking(move || -> Result<(String, Vec<u8>), anyhow::Error> {
                let source = std::fs::File::open(filename)?;

                let mut buf: Vec<u8> = vec![];
                compress_tools::uncompress_archive_file(source, &mut buf, &path)?;

                Ok((content_type, buf))
            })
            .await?
        }
    }

    #[cfg(feature = "server")]
    async fn get_image_from_url_stream(
        &self,
        url: &str,
    ) -> Result<http::Response<Body>, StatusCode> {
        debug!("get image from {}", url);
        if url.is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let source_res = match self.client.get(url).send().await {
            Ok(res) => res,
            Err(e) => {
                error!("error fetch image, reason: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        let mut res = http::Response::builder().status(source_res.status());

        for (name, value) in source_res.headers() {
            res = res.header(name, value);
        }

        Ok(res
            .body(Body::wrap_stream(source_res.bytes_stream()))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
    }

    #[allow(dead_code)]
    async fn get_image_from_url(&self, url: &str) -> Result<(String, Vec<u8>), anyhow::Error> {
        debug!("get image from {}", url);
        if url.is_empty() {
            return Err(anyhow::anyhow!("url is empty"));
        }

        let data = self.client.get(url).send().await?.bytes().await?.to_vec();

        let content_type = mime_guess::from_path(url)
            .first_or_octet_stream()
            .to_string();

        Ok((content_type, data))
    }

    #[allow(dead_code)]
    pub async fn get_image_raw(&self, url: &str) -> Result<(String, Vec<u8>), anyhow::Error> {
        let url = match utils::decrypt_url(&self.secret, url) {
            Ok(url) => url,
            Err(e) => {
                error!("error validate url: {}", e);
                "".to_string()
            }
        };

        match url {
            url if url.starts_with("http") => Ok(self.get_image_from_url(&url).await?),
            url if !url.is_empty() => Ok(self.get_image_from_file(&url).await?),
            _ => Err(anyhow::anyhow!("failed to get image")),
        }
    }
}
