use rust_embed::RustEmbed;
use warp::{Reply, Rejection};
use warp::path::Tail;
use warp::http::header::HeaderValue;
use warp::reply::Response;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../tanoshi-web/dist/"]
pub struct Asset;

#[derive(Debug, Clone)]
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
    log::info!("encoding: {:?}, path: {}", encoding, path);
    let mut res = if path.ends_with(".js") || path.ends_with(".wasm") {
        match encoding {
            Encoding::Br => {
                let asset = Asset::get(format!("{}.br", path).as_str()).ok_or_else(warp::reject::not_found)?;
                let mut res = Response::new(asset.into());
                res.headers_mut().insert(
                    "Content-Encoding",
                    HeaderValue::from_str("br").unwrap(),
                );
                res
            }
            Encoding::Gzip => {
                let asset = Asset::get(format!("{}.gz", path).as_str()).ok_or_else(warp::reject::not_found)?;
                let mut res = Response::new(asset.into());
                res.headers_mut().insert(
                    "Content-Encoding",
                    HeaderValue::from_str("gzip").unwrap(),
                );
                res
            }
            Encoding::None => {
                let asset = Asset::get(path).ok_or_else(warp::reject::not_found)?;
                Response::new(asset.into())
            }
        }
    } else {
        let asset = Asset::get(path).ok_or_else(warp::reject::not_found)?;
        Response::new(asset.into())
    };
    
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    res.headers_mut().insert(
        "Content-Type",
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );
    Ok(res)
}