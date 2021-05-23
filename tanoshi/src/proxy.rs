use serde::Deserialize;
use std::convert::Infallible;
use warp::Filter;

#[derive(Deserialize)]
pub struct Image {
    pub url: String,
}

pub fn proxy() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("image")
        .and(warp::get())
        .and(warp::query::<Image>())
        .and_then(get_image)
}

pub async fn get_image(image: Image) -> Result<impl warp::Reply, Infallible> {
    info!("get image from {}", image.url);
    if image.url.is_empty() {
        return Ok(warp::http::Response::builder()
            .status(400)
            .body(bytes::Bytes::new())
            .unwrap_or_default());
    }
    let res = reqwest::get(&image.url)
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    let mut content_type = image.url.split('.').rev().take(1);
    let content_type = match content_type.next() {
        Some(ext) => ["image", ext].join("/"),
        None => "application/octet-stream".to_string(),
    };

    Ok(warp::http::Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Length", res.len())
        .header("Cache-Control", "max-age=315360000")
        .body(res)
        .unwrap_or_default())
}
