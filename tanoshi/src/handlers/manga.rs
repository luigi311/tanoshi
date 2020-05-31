use sqlx::postgres::PgPool;
use warp::{http::Response, Rejection};

use serde_json::json;
use std::io::Read;
use ureq;

use crate::auth::Claims;
use crate::extension::{repository, ExtensionProxy, Extensions};
use tanoshi_lib::extensions::Extension;
use tanoshi_lib::manga::{GetParams, ImageProxyParam, Params, Source, SourceLogin};

use crate::handlers::TransactionReject;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

pub async fn list_sources(
    exts: Arc<RwLock<Extensions>>,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let exts = exts.read().await;
    let sources = exts
        .extensions()
        .iter()
        .map(|(key, ext)| {
            info!("source name {}", key.clone());
            ext.info()
        })
        .collect::<Vec<Source>>();
    info!("sources {:?}", sources.clone());

    match repository::insert_sources(sources, db.clone()).await {
        Ok(_) => {}
        Err(e) => return Err(warp::reject()),
    }

    match repository::get_sources(db).await {
        Ok(sources) => Ok(warp::reply::json(&json!(
            {
                "sources": sources,
                "status": "success"
            }
        ))),
        Err(e) => Err(warp::reject::custom(TransactionReject {
            message: e.to_string(),
        })),
    }
}

pub async fn list_mangas(
    source_id: i32,
    claim: Claims,
    param: Params,
    exts: Arc<RwLock<Extensions>>,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let exts = exts.read().await;
    if let Ok(source) = repository::get_source(source_id, db.clone()).await {
        let mangas = exts
            .get(&source.name)
            .unwrap()
            .get_mangas(&source.url, param, vec![])
            .unwrap();

        let manga_ids = match repository::insert_mangas(source_id, mangas.clone(), db.clone()).await
        {
            Ok(ids) => ids,
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        };
        match repository::get_mangas(claim.sub, manga_ids, db).await {
            Ok(mangas) => return Ok(warp::reply::json(&mangas)),
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        }
    }
    Err(warp::reject())
}

pub async fn get_manga_info(
    manga_id: i32,
    claim: Claims,
    exts: Arc<RwLock<Extensions>>,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let exts = exts.read().await;
    if let Ok(manga) = repository::get_manga_detail(manga_id, claim.sub.clone(), db.clone()).await {
        return Ok(warp::reply::json(&manga));
    } else if let Ok(url) = repository::get_manga_url(manga_id, db.clone()).await {
        let source = match repository::get_source_from_manga_id(manga_id, db.clone()).await {
            Ok(source) => source,
            Err(e) => return Err(warp::reject()),
        };

        let manga = exts
            .get(&source.name)
            .unwrap()
            .get_manga_info(&url)
            .unwrap();

        match repository::update_manga_info(manga_id, manga, db.clone()).await {
            Ok(_) => {}
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        }
        match repository::get_manga_detail(manga_id, claim.sub, db).await {
            Ok(res) => return Ok(warp::reply::json(&res)),
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        }
    }
    Err(warp::reject())
}

pub async fn get_chapters(
    manga_id: i32,
    claim: Claims,
    param: GetParams,
    exts: Arc<RwLock<Extensions>>,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let exts = exts.read().await;
    if !param.refresh.unwrap_or(false) {
        match repository::get_chapters(manga_id, claim.sub.clone(), db.clone()).await {
            Ok(chapter) => return Ok(warp::reply::json(&chapter)),
            Err(_e) => {}
        };
    }

    if let Ok(url) = repository::get_manga_url(manga_id, db.clone()).await {
        let source = match repository::get_source_from_manga_id(manga_id, db.clone()).await {
            Ok(source) => source,
            Err(e) => return Err(warp::reject()),
        };

        let chapter = exts.get(&source.name).unwrap().get_chapters(&url).unwrap();

        match repository::insert_chapters(manga_id, chapter.clone(), db.clone()).await {
            Ok(_) => {}
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        }

        match repository::get_chapters(manga_id, claim.sub, db.clone()).await {
            Ok(chapter) => return Ok(warp::reply::json(&chapter)),
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        };
    }
    Err(warp::reject())
}

pub async fn get_pages(
    chapter_id: i32,
    _param: GetParams,
    exts: Arc<RwLock<Extensions>>,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let exts = exts.read().await;
    match repository::get_pages(chapter_id, db.clone()).await {
        Ok(pages) => return Ok(warp::reply::json(&pages)),
        Err(_) => {}
    };

    if let Ok(url) = repository::get_chapter_url(chapter_id, db.clone()).await {
        let source = match repository::get_source_from_chapter_id(chapter_id, db.clone()).await {
            Ok(source) => source,
            Err(e) => return Err(warp::reject()),
        };

        let pages = exts.get(&source.name).unwrap().get_pages(&url).unwrap();

        match repository::insert_pages(chapter_id, pages.clone(), db.clone()).await {
            Ok(_) => {}
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        }

        match repository::get_pages(chapter_id, db.clone()).await {
            Ok(pages) => return Ok(warp::reply::json(&pages)),
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        };
    }
    Err(warp::reject())
}

pub async fn proxy_image(
    param: ImageProxyParam,
    exts: Arc<RwLock<Extensions>>,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let url = param.url.clone();
    let source = match repository::get_source_from_image_url(url.clone(), db.clone()).await {
        Ok(source) => source,
        Err(e) => return Err(warp::reject()),
    };

    let mut bytes = vec![];

    let exts = exts.read().await;
    let content_type = exts
        .get(&source.name)
        .unwrap()
        .get_page(&url, &mut bytes)
        .unwrap();

    let resp = Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Length", bytes.len())
        .body(bytes)
        .unwrap();

    Ok(resp)
}

pub async fn source_login(
    source: String,
    login_info: SourceLogin,
    exts: Arc<RwLock<Extensions>>,
) -> Result<impl warp::Reply, Rejection> {
    let exts = exts.read().await;
    let result = match exts.get(&source).unwrap().login(login_info) {
        Ok(result) => result,
        Err(e) => return Err(warp::reject()),
    };
    Ok(warp::reply::json(&result))
}
