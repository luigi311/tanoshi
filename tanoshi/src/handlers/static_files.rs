use rust_embed::RustEmbed;
use warp::{Reply, Rejection};
use warp::path::Tail;
use warp::http::header::HeaderValue;
use warp::reply::Response;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../tanoshi-web/dist/"]
pub struct Asset;

#[derive(Clone)]
pub enum Encoding {
    Br,
    Gzip,
    None,
}

pub async fn serve_index(encoding: Encoding) -> Result<impl Reply, Rejection> {
    serve_impl("index.html", encoding)
}

pub async fn serve_tail(path: Tail, encoding: Encoding) -> Result<impl Reply, Rejection> {
    serve_impl(path.as_str(), encoding)
}

pub fn serve_impl(path: &str, encoding: Encoding) -> Result<impl Reply, Rejection> {
    let asset = Asset::get(path).ok_or_else(warp::reject::not_found)?;

    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let mut res = Response::new(asset.into());
    res.headers_mut().insert(
        "Content-Type",
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );
    Ok(res)
}