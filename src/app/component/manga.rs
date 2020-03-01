use serde::{Deserialize, Serialize};
use stdweb::web::event::IEvent;
use wasm_bindgen::__rt::core::time::Duration;
use yew::{Component, ComponentLink, html, Html, MouseDownEvent, MouseUpEvent, Properties, ShouldRender, TouchEnd, TouchStart, ClickEvent};
use yew::format::{Json, Text, Nothing};
use yew::services::{FetchService, StorageService, Task, TimeoutService};
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew_router::components::RouterAnchor;

use crate::app::{AppRoute, GetChaptersResponse};
use crate::app::home::Msg::FavoritesReady;
use stdweb::web::document;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FavoriteManga {
    pub source: String,
    pub title: String,
    pub path: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AddFavoritesResponse {
    pub status: String,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub title: String,
    pub thumbnail: String,
    pub path: String,
    pub source: String,
    pub is_favorite: bool,
}

pub struct Manga {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    timeout: TimeoutService,
    job: Option<Box<dyn Task>>,
    title: String,
    thumbnail: String,
    path: String,
    pub source: String,
    pub is_favorite: bool,
    token: String,
}

pub enum Msg {
    Click(ClickEvent),
    MouseDown(MouseDownEvent),
    MouseUp(MouseUpEvent),
    MouseDownTimeout,
    TouchStart(TouchStart),
    TouchEnd(TouchEnd),
    Favorited(AddFavoritesResponse),
    Noop,
}

impl Component for Manga {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local);
        let token = {
            if let Ok(token) = storage.restore("token") {
                token
            } else {
                "".to_string()
            }
        };
        Manga {
            fetch_task: None,
            link,
            timeout: TimeoutService::new(),
            job: None,
            title: props.title,
            thumbnail: props.thumbnail,
            path: props.path,
            source: props.source,
            is_favorite: props.is_favorite,
            token,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Click(e) => {
                e.prevent_default();
            }
            Msg::MouseDown(e) => {
                info!("mouse down");
                let handle = self.timeout.spawn(
                    Duration::from_secs(1),
                    self.link.callback(|_| Msg::MouseDownTimeout));
                self.job = Some(Box::new(handle));
            }
            Msg::MouseUp(e) => {
                e.prevent_default();
                info!("mouse up");
                if !self.job.is_none() {

                    self.job = None;
                }
            }
            Msg::MouseDownTimeout => {
                info!("timeout");
                self.favorite();
                self.job = None;
            }
            Msg::Favorited(data) => {
                if data.status == "success" {
                    self.is_favorite = !self.is_favorite;
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
        let path = self.path.to_owned();
        let source = self.source.to_owned();

        html! {
                <div class="col">
                    <div
                    class={if self.is_favorite {"manga-cover-container favorite"} else {"manga-cover-container"}}
                    onmousedown=self.link.callback(|e| Msg::MouseDown(e))
                    onmouseup=self.link.callback(|e| Msg::MouseUp(e))
                    onclick=self.link.callback(|e| Msg::Click(e))
                    >
                        <img class="manga-cover" src=thumbnail/>
                    </div>
                </div>
        }
    }
}

impl Manga {
    fn favorite(&mut self) {
        let fav = FavoriteManga {
            source: self.source.clone(),
            title: self.title.clone(),
            path: self.path.clone(),
            thumbnail_url: self.thumbnail.clone(),
        };

        let req = Request::post("/api/favorites")
            .header("Authorization", self.token.to_owned())
            .header("Content-Type", "application/json")
            .body(Json(&fav))
            .expect("failed to build request");

        let task = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Json<Result<AddFavoritesResponse, anyhow::Error>>>| {
                if let (meta, Json(Ok(data))) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::Favorited(data);
                    }
                }
                Msg::Noop
            }));
        self.fetch_task = Some(task);
    }

    fn unfavorite(&mut self) {
        let req = Request::delete(format!("/api/favorites/source/{}/title/{}", self.title.clone(), self.title.clone()))
            .header("Authorization", self.token.to_owned())
            .body(Nothing)
            .expect("failed to build request");

        let task = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Json<Result<AddFavoritesResponse, anyhow::Error>>>| {
                if let (meta, Json(Ok(data))) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::Favorited(data);
                    }
                }
                Msg::Noop
            }));
        self.fetch_task = Some(task);
    }
}