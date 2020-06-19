use std::sync::Arc;

use tanoshi_lib::manga::{GetParams, Params, SourceLogin};
use tokio::sync::RwLock;
use warp::Rejection;

use crate::auth::Claims;
use crate::extension::manga::Manga;
use crate::extension::Extensions;

pub async fn list_sources(param: String, manga: Manga) -> Result<impl warp::Reply, Rejection> {
    manga.list_sources(param).await
}

pub async fn install_source(
    name: String,
    plugin_path: String,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.install_source(name, plugin_path).await
}

pub async fn list_mangas(
    source_id: i32,
    claim: Claims,
    source_auth: String,
    param: Params,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga
        .list_mangas(source_id, claim, source_auth, param)
        .await
}

pub async fn get_manga_info(
    manga_id: i32,
    claim: Claims,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.get_manga_info(manga_id, claim).await
}

pub async fn get_chapters(
    manga_id: i32,
    claim: Claims,
    param: GetParams,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.get_chapters(manga_id, claim, param).await
}

pub async fn get_pages(
    chapter_id: i32,
    param: GetParams,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.get_pages(chapter_id, param).await
}

pub async fn proxy_image(page_id: i32, manga: Manga) -> Result<impl warp::Reply, Rejection> {
    manga.proxy_image(page_id).await
}

pub async fn source_login(
    source_id: i32,
    login_info: SourceLogin,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.source_login(source_id, login_info).await
}
