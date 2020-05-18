use serde::{Deserialize, Serialize};
use wasm_bindgen::__rt::core::time::Duration;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::{FetchService, StorageService, Task, TimeoutService};
use yew::{html, Bridge, Bridged, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::agent::{RouteAgent, RouteRequest};
use yew_router::prelude::*;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FavoriteManga {
    pub source: String,
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AddFavoritesResponse {
    pub status: String,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub id: i32,
    pub title: String,
    pub thumbnail: String,
    pub is_favorite: bool,
}

pub struct Manga {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    timeout: TimeoutService,
    job: Option<Box<dyn Task>>,
    id: i32,
    router: Box<dyn Bridge<RouteAgent>>,
    title: String,
    thumbnail: String,
    pub is_favorite: bool,
    token: String,
    is_dragging: bool,
}

pub enum Msg {
    Click(MouseEvent),
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseDownTimeout,
    TouchStart(TouchEvent),
    TouchEnd(TouchEvent),
    TouchMove(TouchEvent),
    Favorited(AddFavoritesResponse),
    Unfavorited(AddFavoritesResponse),
    Noop,
}

impl Component for Manga {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let token = {
            if let Ok(token) = storage.restore("token") {
                token
            } else {
                "".to_string()
            }
        };
        let callback = link.callback(|_| Msg::Noop);
        let router = RouteAgent::bridge(callback);
        Manga {
            fetch_task: None,
            link,
            timeout: TimeoutService::new(),
            job: None,
            router,
            id: props.id,
            title: props.title,
            thumbnail: props.thumbnail,
            is_favorite: props.is_favorite,
            token,
            is_dragging: false,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Click(e) => {
                e.prevent_default();
            }
            Msg::MouseDown(e) => {
                self.start_timer();
            }
            Msg::MouseUp(e) => {
                e.prevent_default();
                self.to_detail();
            }
            Msg::TouchStart(e) => {
                self.start_timer();
                self.is_dragging = false;
            }
            Msg::TouchEnd(e) => {
                e.prevent_default();
                if !self.is_dragging {
                    self.to_detail();
                }
            }
            Msg::TouchMove(e) => {
                self.is_dragging = true;
            }
            Msg::MouseDownTimeout => {
                info!("timeout");
                if !self.is_dragging {
                    if self.is_favorite {
                        self.unfavorite();
                    } else {
                        self.favorite();
                    }
                }
                self.job = None;
            }
            Msg::Favorited(data) => {
                if data.status == "success" {
                    self.is_favorite = true;
                }
            }
            Msg::Unfavorited(data) => {
                if data.status == "success" {
                    self.is_favorite = false;
                }
            }
            Msg::Noop => {}
            _ => {}
        }
        true
    }

    fn view(&self) -> Html {
        let _title = self.title.to_owned();
        let thumbnail = self.thumbnail.to_owned();
        html! {
            <div
            class=self.classes()
            onmousedown=self.link.callback(|e| Msg::MouseDown(e))
            onmouseup=self.link.callback(|e| Msg::MouseUp(e))
            ontouchstart=self.link.callback(|e| Msg::TouchStart(e))
            ontouchend=self.link.callback(|e| Msg::TouchEnd(e))
            ontouchmove=self.link.callback(|e| Msg::TouchMove(e))
            onclick=self.link.callback(|e| Msg::Click(e))
            >
                <div class="manga-cover sm:cover cover-sm bg-center bg-cover relative" style={format!("background-image: url({})",thumbnail)}>
                    <span class="absolute sm:title title-sm bottom-0 sm:text-sm text-xs bg-black opacity-75 text-white p-1 truncate">{self.title.clone()}</span>
                </div>
            </div>
        }
    }
}

impl Manga {
    fn classes(&self) -> Vec<&str> {
        let mut classes = vec!["relative", "cursor-pointer"];
        if self.is_favorite {
            classes.push("favorite");
        }
        return classes;
    }

    fn favorite(&mut self) {
        let req = Request::post(format!("/api/favorites/manga/{}", self.id))
            .header("Authorization", self.token.to_owned())
            .header("Content-Type", "application/json")
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<AddFavoritesResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::Favorited(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }

    fn unfavorite(&mut self) {
        let req = Request::delete(format!("/api/favorites/manga/{}", self.id))
        .header("Authorization", self.token.to_owned())
        .body(Nothing)
        .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<AddFavoritesResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::Unfavorited(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
    fn start_timer(&mut self) {
        let handle = self.timeout.spawn(
            Duration::from_secs(1),
            self.link.callback(|_| Msg::MouseDownTimeout),
        );
        self.job = Some(Box::new(handle));
    }

    fn to_detail(&mut self) {
        if !self.job.is_none() {
            self.router
                .send(RouteRequest::ChangeRoute(Route::from(format!(
                    "/manga/{}",
                    self.id
                ))));
            self.job = None;
        }
    }
}
