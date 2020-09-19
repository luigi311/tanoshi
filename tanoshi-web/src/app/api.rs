use anyhow::{anyhow, Error};
use tanoshi_lib::manga::Params;
use tanoshi_lib::rest::GetMangasResponse;
use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::Callback;

pub type FetchJsonResponse<T> = Response<Json<Result<T, Error>>>;
pub type FetchTextResponse = Response<Text>;
type FetchJsonCallback<T> = Callback<FetchJsonResponse<T>>;
type FetchTextCallback = Callback<FetchTextResponse>;

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
