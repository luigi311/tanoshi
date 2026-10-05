use super::*;
use axum::{Router, body::to_bytes, routing::get};
use tower_http::compression::CompressionLayer;

const BUNDLE: &str = "/app-0123456789abcdef.js";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

#[test]
fn only_hashed_bundles_are_immutable() {
    for path in [
        "app-0123456789abcdef.js",
        "app-0123456789abcdef.wasm",
        "app-0123456789abcdef_bg.wasm",
        "styles-0123456789abcdef.css",
    ] {
        assert_eq!(cache_control(path), IMMUTABLE, "{path}");
    }
    for path in [
        "index.html",
        "sw.js",
        "manifest.webmanifest",
        "animate.min.css",
        "icons/tanoshi.png",
        "images/cover-placeholder.jpg",
        "app.js",
        "app-0123456789abcde.js",
        "app-0123456789abcdef0.js",
        "app-0123456789abcdeg.js",
        "app-0123456789abcdef.js.map",
    ] {
        assert_eq!(cache_control(path), "no-cache", "{path}");
    }
}

#[tokio::test]
async fn conditional_requests_reuse_content_and_preserve_cache_headers() {
    let response = static_handler(
        Request::builder()
            .uri("/index.html")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
    assert_eq!(response.headers()[header::CONTENT_TYPE], "text/html");
    let etag = response.headers()[header::ETAG]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(etag.starts_with("W/\""));
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert!(!body.is_empty());

    for value in [
        etag.clone(),
        etag.strip_prefix("W/").unwrap().to_owned(),
        format!("\"older-content\", {etag}"),
        "*".to_owned(),
    ] {
        let response = static_handler(
            Request::builder()
                .uri("/index.html")
                .header(header::IF_NONE_MATCH, value)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()[header::ETAG], etag);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        assert_eq!(response.headers()[header::VARY], "accept-encoding");
        assert!(
            to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
                .is_empty()
        );
    }
    let response = static_handler(
        Request::builder()
            .uri("/index.html")
            .header(header::IF_NONE_MATCH, "\"older-content\"")
            .header(header::IF_NONE_MATCH, &etag)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);

    for value in ["\"older-content\"", "W/\"older-content\"", "invalid"] {
        let response = static_handler(
            Request::builder()
                .uri("/index.html")
                .header(header::IF_NONE_MATCH, value)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), body);
    }
}

#[tokio::test]
async fn spa_fallbacks_revalidate_index_instead_of_caching_the_requested_filename() {
    let response = static_handler(
        Request::builder()
            .uri("/index.html")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let etag = response.headers()[header::ETAG].clone();
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    for path in ["/", "/library", "/missing-0123456789abcdef.js"] {
        let response = static_handler(
            Request::builder()
                .uri(path)
                .header(header::ACCEPT, "text/html")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        assert_eq!(response.headers()[header::ETAG], etag);
        assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), body);
        let response = static_handler(
            Request::builder()
                .uri(path)
                .header(header::ACCEPT, "text/html")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        assert!(
            to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
                .is_empty()
        );
    }
    let response = static_handler(
        Request::builder()
            .uri("/missing.js")
            .header(header::ACCEPT, "application/javascript")
            .header(header::IF_NONE_MATCH, "*")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(!response.headers().contains_key(header::ETAG));
}

#[tokio::test]
async fn compressed_fallbacks_share_weak_etags_and_return_empty_conditional_responses() {
    let router = Router::new().fallback(get(static_handler).layer(CompressionLayer::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::builder()
        .no_gzip()
        .no_brotli()
        .no_zstd()
        .no_deflate()
        .build()
        .unwrap();
    let url = format!("http://{address}{BUNDLE}");
    let original = include_bytes!("assets_test_files/app-0123456789abcdef.js");
    let mut previous_etag = None;
    for encoding in ["gzip", "br", "zstd", "deflate", "identity"] {
        let response = client
            .get(&url)
            .header("accept-encoding", encoding)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
        assert_eq!(response.headers()[header::VARY], "accept-encoding");
        let etag = response.headers()[header::ETAG].clone();
        assert!(etag.to_str().unwrap().starts_with("W/\""));
        if let Some(previous) = &previous_etag {
            assert_eq!(&etag, previous);
        }
        previous_etag = Some(etag.clone());
        if encoding == "identity" {
            assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
            assert_eq!(response.bytes().await.unwrap().as_ref(), original);
        } else {
            assert_eq!(response.headers()[header::CONTENT_ENCODING], encoding);
            assert!(response.bytes().await.unwrap().len() < original.len());
        }
        let response = client
            .get(&url)
            .header("accept-encoding", encoding)
            .header(header::IF_NONE_MATCH, &etag)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()[header::ETAG], etag);
        assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
        assert_eq!(response.headers()[header::VARY], "accept-encoding");
        assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
        assert!(response.bytes().await.unwrap().is_empty());
    }
    let response = client.head(&url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        &response.headers()[header::ETAG],
        previous_etag.as_ref().unwrap()
    );
    assert!(response.bytes().await.unwrap().is_empty());
    let response = client
        .head(&url)
        .header(header::IF_NONE_MATCH, previous_etag.unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert!(response.bytes().await.unwrap().is_empty());
    server.abort();
    let _ = server.await;
}
