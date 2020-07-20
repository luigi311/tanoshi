use tanoshi_lib::manga::{GetParams, Params, SourceLogin};
use warp::Rejection;

use crate::auth::Claims;
use crate::extension::manga::Manga;

use crate::handlers::TransactionReject;

pub async fn list_sources(manga: Manga) -> Result<impl warp::Reply, Rejection> {
    manga.list_sources().await
}

pub async fn install_source(
    name: String,
    plugin_path: String,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.install_source(name, plugin_path).await
}

pub async fn list_mangas(
    source: String,
    claim: Claims,
    source_auth: String,
    param: Params,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    match manga.list_mangas(source, claim, source_auth, param).await {
        Ok(res) => Ok(warp::reply::json(&res)),
        Err(e) => Err(warp::reject::custom(TransactionReject {
            message: e.to_string(),
        })),
    }
}

pub async fn get_manga_info(
    manga_id: i32,
    claim: Claims,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    match manga.get_manga_info(manga_id, claim).await {
        Ok(res) => Ok(warp::reply::json(&res)),
        Err(e) => Err(warp::reject::custom(TransactionReject {
            message: e.to_string(),
        })),
    }
}

pub async fn get_chapters(
    manga_id: i32,
    claim: Claims,
    param: GetParams,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    match manga.get_chapters(manga_id, claim, param).await {
        Ok(res) => Ok(warp::reply::json(&res)),
        Err(e) => Err(warp::reject::custom(TransactionReject {
            message: e.to_string(),
        })),
    }
}

pub async fn get_pages(
    chapter_id: i32,
    param: GetParams,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    match manga.get_pages(chapter_id, param).await {
        Ok(pages) => Ok(warp::reply::json(&pages)),
        Err(e) => Err(warp::reject::custom(TransactionReject {
            message: e.to_string(),
        })),
    }
}

pub async fn proxy_image(page_id: i32, manga: Manga) -> Result<impl warp::Reply, Rejection> {
    manga.proxy_image(page_id).await
}

pub async fn source_login(
    source: String,
    login_info: SourceLogin,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.source_login(source, login_info).await
}

pub async fn read(
    chapter_id: i32,
    claim: Claims,
    param: GetParams,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    match manga.read(chapter_id, claim, param).await {
        Ok(res) => Ok(warp::reply::json(&res)),
        Err(e) => Err(warp::reject::custom(TransactionReject {
            message: e.to_string(),
        })),
    }
}
