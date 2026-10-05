use axum::{
    body::Body,
    http::{StatusCode, header},
    response::Response,
};
use bytes::Bytes;
use headers::{ETag, HeaderMapExt, IfNoneMatch};
use http::Request;
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;

pub async fn static_handler(req: Request<Body>) -> Response {
    let mut path = req.uri().path().trim_start_matches('/');
    let accept = req.headers().get("accept").and_then(|v| v.to_str().ok());
    let content = Asset::get(path).or_else(|| {
        if accept.is_some_and(|header| header.contains("*/*") || header.contains("text/html")) {
            path = "index.html";
            Asset::get(path)
        } else {
            None
        }
    });
    let Some(content) = content else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404"))
            .unwrap();
    };

    // A weak ETag identifies the same content across compression encodings.
    let hash = content.metadata.sha256_hash();
    let etag = format!(
        "W/\"{}\"",
        hash.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let not_modified = req
        .headers()
        .typed_get::<IfNoneMatch>()
        .is_some_and(|value| !value.precondition_passes(&etag.parse::<ETag>().unwrap()));
    let response = Response::builder()
        .header(header::CACHE_CONTROL, cache_control(path))
        .header(header::ETAG, etag)
        .header(header::VARY, "accept-encoding");
    if not_modified {
        return response
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .unwrap();
    }

    let data = match content.data {
        Cow::Borrowed(data) => Bytes::from_static(data),
        Cow::Owned(data) => Bytes::from(data),
    };
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    response
        .header(header::CONTENT_TYPE, mime.as_ref())
        .body(Body::from(data))
        .unwrap()
}

#[derive(RustEmbed)]
#[cfg_attr(not(test), folder = "$CARGO_MANIFEST_DIR/../tanoshi-web/dist")]
#[cfg_attr(
    test,
    folder = "$CARGO_MANIFEST_DIR/src/presentation/assets_test_files"
)]
struct Asset;

fn cache_control(path: &str) -> &'static str {
    let filename = path.rsplit('/').next().unwrap_or(path);
    if let Some((stem, extension)) = filename.rsplit_once('.') {
        let stem = if extension == "wasm" {
            stem.strip_suffix("_bg").unwrap_or(stem)
        } else {
            stem
        };
        if matches!(extension, "js" | "wasm" | "css")
            && stem.rsplit_once('-').is_some_and(|(_, hash)| {
                hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return "public, max-age=31536000, immutable";
        }
    }
    "no-cache"
}
