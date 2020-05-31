use serde::{Deserialize, Serialize};
use yew::worker::*;

use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{FetchService, FetchTask};
use yew::services::storage::{Area, StorageService};

use http::request::Request as HttpRequest;
use http::response::Response as HttpResponse;

use std::collections::HashMap;

use anyhow::Result;

use tanoshi_lib::manga::{
    GetChaptersResponse, GetMangaResponse, GetMangasResponse, GetPagesResponse, HistoryRequest,
    Manga as MangaModel, Params, SortByParam, SortOrderParam, Source as SourceModel,
};

#[derive(Deserialize, Serialize)]
pub enum Request {
    PostHistory(String, HistoryRequest),
    FetchMangas(i32, Params),
    FetchManga(i32),
    FetchChapters(i32, bool),
    FetchPages(i32),
}

#[derive(Deserialize, Serialize)]
pub enum Response {
    HistoryPosted,
    MangasFetched(GetMangasResponse),
    MangaFetched(GetMangaResponse),
    ChaptersFetched(GetChaptersResponse),
    PagesFetched(GetPagesResponse),
}

pub struct Worker {
    link: AgentLink<Worker>,
    fetch_task: HashMap<HandlerId, FetchTask>,
    storage: StorageService,
    token: String,
}

pub enum Msg {
    HistoryPosted(HandlerId),
    MangasReady(HandlerId, GetMangasResponse),
    MangaReady(HandlerId, GetMangaResponse),
    ChaptersReady(HandlerId, GetChaptersResponse),
    PagesReady(HandlerId, GetPagesResponse),
    Noop,
}

impl Agent for Worker {
    type Reach = Context;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        info!("create");
        let storage = StorageService::new(Area::Local).unwrap();
        let token = storage.restore::<Result<String>>("token").unwrap();

        Worker {
            link,
            fetch_task: HashMap::new(),
            storage,
            token,
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::HistoryPosted(id) => {
                self.fetch_task.remove(&id.clone());
                self.link.respond(id, Response::HistoryPosted);
            }
            Msg::MangasReady(id, data) => {
                self.fetch_task.remove(&id.clone());
                self.link.respond(id, Response::MangasFetched(data));
            }
            Msg::MangaReady(id, data) => {
                self.fetch_task.remove(&id.clone());
                self.link.respond(id, Response::MangaFetched(data));
            }
            Msg::ChaptersReady(id, data) => {
                self.fetch_task.remove(&id.clone());
                self.link.respond(id, Response::ChaptersFetched(data));
            }
            Msg::PagesReady(id, data) => {
                self.fetch_task.remove(&id.clone());
                self.link.respond(id, Response::PagesFetched(data));
            }
            Msg::Noop => {}
        }
    }

    fn handle_input(&mut self, msg: Self::Input, id: HandlerId) {
        match msg {
            Request::PostHistory(token, request) => {
                let req = HttpRequest::post("/api/history")
                    .header("Authorization", token.to_string())
                    .header("Content-Type", "application/json")
                    .body(Json(&request))
                    .expect("failed to build request");

                if let Ok(task) = FetchService::new().fetch(
                    req,
                    self.link.callback(move |response: HttpResponse<Text>| {
                        if let (meta, Ok(data)) = response.into_parts() {
                            if meta.status.is_success() {
                                return Msg::HistoryPosted(id);
                            }
                        }
                        Msg::Noop
                    }),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
            Request::FetchMangas(source_id, params) => {
                let params = serde_urlencoded::to_string(params).unwrap();

                let source_auth = self
                    .storage
                    .restore::<Result<String>>("SourceAuthorization")
                    .unwrap_or("".to_string());

                let source_auth =
                    serde_json::from_str(&source_auth).unwrap_or(HashMap::<i32, String>::new());

                let req = HttpRequest::get(format!("/api/source/{}?{}", source_id, params))
                    .header("Authorization", self.token.clone())
                    .header(
                        "source-token",
                        source_auth
                            .get(&source_id)
                            .unwrap_or(&"".to_string())
                            .as_str(),
                    )
                    .body(Nothing)
                    .expect("failed to build request");

                if let Ok(task) = FetchService::new().fetch(
                    req,
                    self.link.callback(
                        move |response: HttpResponse<Json<Result<GetMangasResponse>>>| {
                            if let (meta, Json(Ok(data))) = response.into_parts() {
                                if meta.status.is_success() {
                                    return Msg::MangasReady(id, data);
                                }
                            }
                            Msg::Noop
                        },
                    ),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
            Request::FetchManga(manga_id) => {
                let req = HttpRequest::get(format!("/api/manga/{}", manga_id))
                    .header("Authorization", self.token.to_string())
                    .body(Nothing)
                    .expect("failed to build request");

                if let Ok(task) = FetchService::new().fetch(
                    req,
                    self.link.callback(
                        move |response: HttpResponse<Json<Result<GetMangaResponse>>>| {
                            if let (meta, Json(Ok(data))) = response.into_parts() {
                                if meta.status.is_success() {
                                    return Msg::MangaReady(id, data);
                                }
                            }
                            Msg::Noop
                        },
                    ),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
            Request::FetchChapters(manga_id, refresh) => {
                let req = HttpRequest::get(format!(
                    "/api/manga/{}/chapter?refresh={}",
                    manga_id, refresh
                ))
                .header("Authorization", self.token.to_string())
                .body(Nothing)
                .expect("failed to build request");

                if let Ok(task) = FetchService::new().fetch(
                    req,
                    self.link.callback(
                        move |response: HttpResponse<Json<Result<GetChaptersResponse>>>| {
                            if let (meta, Json(Ok(data))) = response.into_parts() {
                                if meta.status.is_success() {
                                    return Msg::ChaptersReady(id, data);
                                }
                            }
                            Msg::Noop
                        },
                    ),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
            Request::FetchPages(chapter_id) => {
                let req = HttpRequest::get(format!("/api/chapter/{}", chapter_id))
                    .body(Nothing)
                    .expect("failed to build request");

                if let Ok(task) = FetchService::new().fetch(
                    req,
                    self.link.callback(
                        move |response: HttpResponse<Json<Result<GetPagesResponse>>>| {
                            if let (meta, Json(Ok(data))) = response.into_parts() {
                                if meta.status.is_success() {
                                    return Msg::PagesReady(id, data);
                                }
                            }
                            Msg::Noop
                        },
                    ),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
        }
    }

    fn connected(&mut self, _id: HandlerId) {
        info!("connected");
    }

    fn disconnected(&mut self, _id: HandlerId) {
        info!("disconnected");
    }

    fn destroy(&mut self) {
        info!("destroyed");
    }
}
