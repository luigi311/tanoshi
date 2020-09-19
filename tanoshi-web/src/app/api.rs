use anyhow::{anyhow, Error};
use tanoshi_lib::manga::{Params, SourceLogin, SourceLoginResult};
use tanoshi_lib::rest::{
    GetChaptersResponse, GetMangaResponse, GetMangasResponse, GetPagesResponse, HistoryRequest,
    ReadResponse,
};
use yew::format::{Binary, Json, Nothing, Text};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::Callback;

pub type FetchJsonResponse<T> = Response<Json<Result<T, Error>>>;
pub type FetchTextResponse = Response<Text>;
pub type FetchBinaryResponse = Response<Binary>;

type FetchJsonCallback<T> = Callback<FetchJsonResponse<T>>;
type FetchTextCallback = Callback<FetchTextResponse>;
type FetchBinaryCallback = Callback<FetchBinaryResponse>;

pub fn get_local_storage() -> Result<StorageService, Error> {
    match StorageService::new(Area::Local) {
        Ok(storage) => Ok(storage),
        Err(e) => Err(anyhow!(e)),
    }
}
pub fn get_token() -> Result<String, Error> {
    let storage = get_local_storage()?;
    storage.restore("token")
}

pub fn get_source_auth(source_name: &str) -> Result<String, Error> {
    let storage = get_local_storage()?;
    storage.restore(format!("source-token-{}", source_name).as_str())
}

pub fn validate_token(callback: FetchTextCallback) -> Result<FetchTask, Error> {
    let token = get_token()?;
    let req = Request::get("/api/validate")
        .header("Authorization", token)
        .body(Nothing)
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn fetch_manga_chapter(manga_id: i32, callback: FetchTextCallback) -> Result<FetchTask, Error> {
    let token = get_token()?;
    let req = Request::get(format!("/api/manga/{}/chapter?refresh=true", manga_id))
        .header("Authorization", token)
        .body(Nothing)
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn fetch_favorites(
    params: Params,
    callback: FetchJsonCallback<GetMangasResponse>,
) -> Result<FetchTask, Error> {
    let token = get_token()?;
    let params = serde_urlencoded::to_string(params).unwrap();
    let req = Request::get(format!("/api/favorites?{}", params))
        .header("Authorization", token)
        .body(Nothing)
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn post_history(
    request: HistoryRequest,
    callback: FetchTextCallback,
) -> Result<FetchTask, Error> {
    let token = get_token()?;
    let req = Request::post("/api/history")
        .header("Authorization", token)
        .header("Content-Type", "application/json")
        .body(Json(&request))
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn fetch_read(
    chapter_id: i32,
    refresh: bool,
    callback: FetchJsonCallback<ReadResponse>,
) -> Result<FetchTask, Error> {
    let token = get_token()?;
    let req = Request::get(format!("/api/read/{}?refresh={}", chapter_id, refresh))
        .header("Authorization", token)
        .body(Nothing)
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn fetch_mangas(
    source_name: &str,
    params: Params,
    callback: FetchJsonCallback<GetMangasResponse>,
) -> Result<FetchTask, Error> {
    let params = serde_urlencoded::to_string(params)?;
    let token = get_token()?;
    let source_auth = get_source_auth(source_name)?;

    let req = Request::get(format!("/api/source/{}?{}", source_name, params))
        .header("Authorization", token)
        .header("SourceAuthorization", source_auth.as_str())
        .body(Nothing)
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn fetch_manga(
    manga_id: i32,
    callback: FetchJsonCallback<GetMangaResponse>,
) -> Result<FetchTask, Error> {
    let token = get_token()?;
    let req = Request::get(format!("/api/manga/{}", manga_id))
        .header("Authorization", token)
        .body(Nothing)
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn fetch_chapters(
    manga_id: i32,
    refresh: bool,
    callback: FetchJsonCallback<GetChaptersResponse>,
) -> Result<FetchTask, Error> {
    let token = get_token()?;
    let req = Request::get(format!(
        "/api/manga/{}/chapter?refresh={}",
        manga_id, refresh
    ))
    .header("Authorization", token)
    .body(Nothing)
    .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn fetch_pages(
    chapter_id: i32,
    refresh: bool,
    callback: FetchJsonCallback<GetPagesResponse>,
) -> Result<FetchTask, Error> {
    let req = Request::get(format!("/api/chapter/{}?refresh={}", chapter_id, refresh))
        .body(Nothing)
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}

pub fn fetch_page(path: &str, callback: FetchBinaryCallback) -> Result<FetchTask, Error> {
    let req = Request::get(path)
        .body(Nothing)
        .expect("failed to build request");

    FetchService::fetch_binary(req, callback)
}

pub fn post_source_login(
    source_name: &str,
    login: SourceLogin,
    callback: FetchJsonCallback<SourceLoginResult>,
) -> Result<FetchTask, Error> {
    let token = get_token()?;
    let req = Request::post(format!("/api/login/{}", source_name))
        .header("Authorization", token)
        .header("Content-Type", "application/json")
        .body(Json(&login))
        .expect("failed to build request");

    FetchService::fetch(req, callback)
}
