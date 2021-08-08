use bytes::Bytes;
use serde::Deserialize;
use std::convert::Infallible;
use warp::{filters::BoxedFilter, hyper::Response, Filter, Reply};

#[derive(Deserialize)]
pub struct Image {
    pub url: String,
}

pub fn proxy() -> BoxedFilter<(impl Reply,)> {
    warp::path!("image")
        .and(warp::get())
        .and(warp::query::<Image>())
        .and_then(get_image)
        .boxed()
}

pub async fn get_image(image: Image) -> Result<impl warp::Reply, Infallible> {
    match image.url {
        url if url.starts_with("http") => Ok(get_image_from_url(url).await?),
        url if !url.is_empty() => Ok(get_image_from_file(url).await?),
        _ => Ok(warp::http::Response::builder()
            .status(400)
            .body(Bytes::new())
            .unwrap()),
    }
}

pub async fn get_image_from_file(file: String) -> Result<Response<Bytes>, Infallible> {
    let file = std::path::PathBuf::from(file);
    let filename = file.parent().unwrap().to_str().unwrap();
    let path = file.file_name().unwrap().to_str().unwrap();
    match libarchive_rs::extract_archive_file(filename, path) {
        Ok(buf) => Ok(warp::http::Response::builder()
            .status(200)
            .body(Bytes::from(buf))
            .unwrap()),
        Err(_) => Ok(warp::http::Response::builder()
            .status(400)
            .body(Bytes::new())
            .unwrap()),
    }
}

pub async fn get_image_from_url(url: String) -> Result<Response<Bytes>, Infallible> {
    debug!("get image from {}", url);
    if url.is_empty() {
        return Ok(warp::http::Response::builder()
            .status(400)
            .body(bytes::Bytes::new())
            .unwrap_or_default());
    }

    let res = match reqwest::get(&url).await.unwrap().bytes().await {
        Ok(res) => res,
        Err(e) => {
            error!("error fetch image, reason: {}", e);
            return Ok(warp::http::Response::builder()
                .status(500)
                .body(bytes::Bytes::new())
                .unwrap_or_default());
        }
    };

    let content_type = match url.split('.').rev().take(1).next() {
        Some(ext) => ["image", ext].join("/"),
        None => "application/octet-stream".to_string(),
    };

    match warp::http::Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Length", res.len())
        .header("Cache-Control", "max-age=315360000")
        .body(res)
    {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("error create response, reason: {}", e);
            return Ok(warp::http::Response::builder()
                .status(500)
                .body(bytes::Bytes::new())
                .unwrap_or_default());
        }
    }
}
