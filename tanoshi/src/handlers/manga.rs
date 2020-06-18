use std::sync::Arc;

use tanoshi_lib::manga::{GetParams, Params, SourceLogin};
use tokio::sync::RwLock;
use warp::Rejection;

use crate::auth::Claims;
use crate::extension::manga::Manga;
use crate::extension::Extensions;

pub async fn list_sources(
    param: String,
    exts: Arc<RwLock<Extensions>>,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.list_sources(param, exts).await
}

pub async fn install_source(
    name: String,
    exts: Arc<RwLock<Extensions>>,
    plugin_path: String,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.install_source(name, exts, plugin_path).await
}

pub async fn list_mangas(
    source_id: i32,
    claim: Claims,
    source_auth: String,
    param: Params,
    exts: Arc<RwLock<Extensions>>,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga
        .list_mangas(source_id, claim, source_auth, param, exts)
        .await
}

pub async fn get_manga_info(
    manga_id: i32,
    claim: Claims,
    exts: Arc<RwLock<Extensions>>,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.get_manga_info(manga_id, claim, exts).await
}

pub async fn get_chapters(
    manga_id: i32,
    claim: Claims,
    param: GetParams,
    exts: Arc<RwLock<Extensions>>,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.get_chapters(manga_id, claim, param, exts).await
}

pub async fn get_pages(
    chapter_id: i32,
    param: GetParams,
    exts: Arc<RwLock<Extensions>>,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.get_pages(chapter_id, param, exts).await
}

pub async fn proxy_image(
    page_id: i32,
    exts: Arc<RwLock<Extensions>>,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.proxy_image(page_id, exts).await
}

pub async fn source_login(
    source_id: i32,
    login_info: SourceLogin,
    exts: Arc<RwLock<Extensions>>,
    manga: Manga,
) -> Result<impl warp::Reply, Rejection> {
    manga.source_login(source_id, login_info, exts).await
}
