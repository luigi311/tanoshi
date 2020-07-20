use serde::{Deserialize, Serialize};
use yew::worker::*;

use yew::format::{Binary, Json, Nothing, Text};
use yew::services::fetch::{FetchService, FetchTask};
use yew::services::storage::{Area, StorageService};

use http::request::Request as HttpRequest;
use http::response::Response as HttpResponse;

use std::collections::HashMap;

use anyhow::Result;

use tanoshi_lib::manga::{Params, SourceLogin, SourceLoginResult};
use tanoshi_lib::rest::{
    GetChaptersResponse, GetMangaResponse, GetMangasResponse, GetPagesResponse, HistoryRequest,
    ReadResponse,
};

#[derive(Deserialize, Serialize)]
pub enum Request {
    PostHistory(String, HistoryRequest),
    FetchMangas(String, Params),
    FetchManga(i32),
    FetchChapters(i32, bool),
    FetchPages(i32, bool),
    FetchPage(String),
    PostLogin(String, SourceLogin),
    FetchRead(i32, bool),
    ValidateToken,
}

#[derive(Deserialize, Serialize)]
pub enum Response {
    HistoryPosted,
    MangasFetched(GetMangasResponse),
    MangaFetched(GetMangaResponse),
    ChaptersFetched(GetChaptersResponse),
    PagesFetched(GetPagesResponse),
    PageFetched(Vec<u8>),
    LoginPosted(SourceLoginResult),
    ReadFetched(ReadResponse),
    TokenInvalidorExpired,
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
    PageReady(HandlerId, Vec<u8>),
    LoginReady(HandlerId, SourceLoginResult),
    ReadReady(HandlerId, ReadResponse),
    ValidateTokenReady(HandlerId),
    Noop,
}

impl Agent for Worker {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let token = storage
            .restore::<Result<String>>("token")
            .unwrap_or("".to_string());

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
            Msg::PageReady(id, data) => {
                self.fetch_task.remove(&id.clone());
                self.link.respond(id, Response::PageFetched(data));
            }
            Msg::LoginReady(id, data) => {
                self.fetch_task.remove(&id.clone());
                self.storage.store(
                    format!("source-token-{}", &data.clone().source_name).as_str(),
                    Ok(data.clone().value),
                );
                self.link.respond(id, Response::LoginPosted(data));
            }
            Msg::ReadReady(id, data) => {
                self.fetch_task.remove(&id.clone());
                self.link.respond(id, Response::ReadFetched(data));
            }
            Msg::ValidateTokenReady(id) => {
                self.fetch_task.remove(&id.clone());
                self.link.respond(id, Response::TokenInvalidorExpired);
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

                if let Ok(task) = FetchService::fetch(
                    req,
                    self.link.callback(move |response: HttpResponse<Text>| {
                        if let (meta, Ok(_)) = response.into_parts() {
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
            Request::FetchMangas(source_name, params) => {
                let params = serde_urlencoded::to_string(params).unwrap();

                let source_auth = self
                    .storage
                    .restore::<Result<String>>(format!("source-token-{}", source_name).as_str())
                    .unwrap_or("".to_string());

                let req = HttpRequest::get(format!("/api/source/{}?{}", source_name, params))
                    .header("Authorization", self.token.clone())
                    .header("SourceAuthorization", source_auth.as_str())
                    .body(Nothing)
                    .expect("failed to build request");

                if let Ok(task) = FetchService::fetch(
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

                if let Ok(task) = FetchService::fetch(
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

                if let Ok(task) = FetchService::fetch(
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
            Request::FetchPages(chapter_id, refresh) => {
                let req =
                    HttpRequest::get(format!("/api/chapter/{}?refresh={}", chapter_id, refresh))
                        .body(Nothing)
                        .expect("failed to build request");

                if let Ok(task) = FetchService::fetch(
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
            Request::FetchPage(path) => {
                let req = HttpRequest::get(path)
                    .body(Nothing)
                    .expect("failed to build request");

                if let Ok(task) = FetchService::fetch_binary(
                    req,
                    self.link.callback(move |response: HttpResponse<Binary>| {
                        if let (meta, Ok(data)) = response.into_parts() {
                            if meta.status.is_success() {
                                return Msg::PageReady(id, data);
                            }
                        }
                        Msg::Noop
                    }),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
            Request::PostLogin(source_name, login) => {
                let req = HttpRequest::post(format!("/api/login/{}", source_name))
                    .header("Authorization", self.token.to_string())
                    .header("Content-Type", "application/json")
                    .body(Json(&login))
                    .expect("failed to build request");

                if let Ok(task) = FetchService::fetch(
                    req,
                    self.link.callback(
                        move |response: HttpResponse<Json<Result<SourceLoginResult>>>| {
                            if let (meta, Json(Ok(data))) = response.into_parts() {
                                if meta.status.is_success() {
                                    return Msg::LoginReady(id, data);
                                }
                            }
                            Msg::Noop
                        },
                    ),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
            Request::FetchRead(chapter_id, refresh) => {
                let req = HttpRequest::get(format!("/api/read/{}?refresh={}", chapter_id, refresh))
                    .header("Authorization", self.token.to_string())
                    .body(Nothing)
                    .expect("failed to build request");

                if let Ok(task) = FetchService::fetch(
                    req,
                    self.link.callback(
                        move |response: HttpResponse<Json<Result<ReadResponse>>>| {
                            if let (meta, Json(Ok(data))) = response.into_parts() {
                                if meta.status.is_success() {
                                    return Msg::ReadReady(id, data);
                                }
                            }
                            Msg::Noop
                        },
                    ),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
            Request::ValidateToken => {
                let req = HttpRequest::get("/api/validate")
                    .header("Authorization", self.token.clone())
                    .body(Nothing)
                    .expect("failed to build request");

                if let Ok(task) = FetchService::fetch(
                    req,
                    self.link.callback(move |response: HttpResponse<Text>| {
                        let (meta, _res) = response.into_parts();
                        let status = meta.status;
                        if status == http::StatusCode::UNAUTHORIZED {
                            return Msg::ValidateTokenReady(id);
                        }
                        Msg::Noop
                    }),
                ) {
                    self.fetch_task.insert(id.clone(), FetchTask::from(task));
                }
            }
        }
    }

    fn connected(&mut self, _id: HandlerId) {
        self.token = self
            .storage
            .restore::<Result<String>>("token")
            .unwrap_or("".to_string());
    }

    fn disconnected(&mut self, _id: HandlerId) {}

    fn destroy(&mut self) {}
}
