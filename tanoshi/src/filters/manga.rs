// use crate::auth::auth::Auth;
// use crate::auth::Claims;
// use crate::filters::settings::settings::auth_handler;
// use crate::filters::with_db;
use crate::filters::with_authorization;
use crate::handlers::manga;

use crate::extension::manga::Manga;

use tanoshi_lib::manga::{GetParams, Params, SourceLogin};
use warp::{filters::BoxedFilter, Filter, Reply};

pub fn manga(secret: String, plugin_path: String, manga: Manga) -> BoxedFilter<(impl Reply,)> {
    list_sources(manga.clone())
        .or(list_mangas(secret.clone(), manga.clone()))
        .or(get_manga_info(secret.clone(), manga.clone()))
        .or(get_chapters(secret.clone(), manga.clone()))
        .or(get_pages(manga.clone()))
        .or(proxy_image(manga.clone()))
        .or(source_login(manga.clone()))
        .or(install_source(plugin_path, manga.clone()))
        .boxed()
}

pub fn list_sources(manga: Manga) -> BoxedFilter<(impl Reply,)> {
    warp::path!("api" / "source" / String)
        .and(warp::get())
        .and(with_manga(manga))
        .and_then(manga::list_sources)
        .boxed()
}

pub fn install_source(plugin_path: String, manga: Manga) -> BoxedFilter<(impl Reply,)> {
    warp::path!("api" / "source" / "install" / String)
        .and(warp::post())
        .and(with_plugin_path(plugin_path))
        .and(with_manga(manga))
        .and_then(manga::install_source)
        .boxed()
}

pub fn list_mangas(secret: String, manga: Manga) -> BoxedFilter<(impl Reply,)> {
    warp::path!("api" / "source" / i32)
        .and(warp::get())
        .and(with_authorization(secret))
        .and(with_source_authorization())
        .and(warp::query::<Params>())
        .and(with_manga(manga))
        .and_then(manga::list_mangas)
        .boxed()
}

pub fn get_manga_info(secret: String, manga: Manga) -> BoxedFilter<(impl Reply,)> {
    warp::path!("api" / "manga" / i32)
        .and(warp::get())
        .and(with_authorization(secret))
        .and(with_manga(manga))
        .and_then(manga::get_manga_info)
        .boxed()
}

pub fn get_chapters(secret: String, manga: Manga) -> BoxedFilter<(impl Reply,)> {
    warp::path!("api" / "manga" / i32 / "chapter")
        .and(warp::get())
        .and(with_authorization(secret))
        .and(warp::query::<GetParams>())
        .and(with_manga(manga))
        .and_then(manga::get_chapters)
        .boxed()
}

pub fn get_pages(manga: Manga) -> BoxedFilter<(impl Reply,)> {
    warp::path!("api" / "chapter" / i32)
        .and(warp::get())
        .and(warp::query::<GetParams>())
        .and(with_manga(manga))
        .and_then(manga::get_pages)
        .boxed()
}

pub fn proxy_image(manga: Manga) -> BoxedFilter<(impl Reply,)> {
    warp::path!("api" / "page" / i32)
        .and(warp::get())
        .and(with_manga(manga))
        .and_then(manga::proxy_image)
        .boxed()
}

pub fn source_login(manga: Manga) -> BoxedFilter<(impl Reply,)> {
    warp::path!("api" / "login" / i32)
        .and(warp::post())
        .and(json_body())
        .and(with_manga(manga))
        .and_then(manga::source_login)
        .boxed()
}

fn json_body() -> impl Filter<Extract = (SourceLogin,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

fn with_plugin_path(
    plugin_path: String,
) -> impl Filter<Extract = (String,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || plugin_path.clone())
}

fn with_manga(
    manga: Manga,
) -> impl Filter<Extract = (Manga,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || manga.clone())
}

pub fn with_source_authorization(
) -> impl Filter<Extract = (String,), Error = warp::reject::Rejection> + Clone {
    warp::header::header("sourceauthorization").map(move |token: String| token.clone())
}
