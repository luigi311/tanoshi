use crate::app::AppRoute;
use serde::Deserialize;
use web_sys::HtmlElement;
use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::{FetchService, StorageService};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use super::component::model::{FavoriteManga, GetFavoritesResponse, GetMangasResponse};
use super::component::{Manga, Spinner};
use yew::utils::{document, window};

use crate::app::component::model::{UpdateModel, UpdatesResponse};
use chrono::Local;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Properties)]
pub struct Props {}

#[derive(Deserialize)]
pub struct Token {
    token: String,
}

pub struct Updates {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    updates: Vec<UpdateModel>,
    token: String,
    is_fetching: bool,
    closure: Closure<dyn Fn()>,
    page: i32,
}

pub enum Msg {
    UpdatesReady(UpdatesResponse),
    ScrolledDown,
    Noop,
}

impl Component for Updates {
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
        let tmp_link = link.clone();
        let closure = Closure::wrap(Box::new(move || {
            let current_scroll = window().scroll_y().expect("error get scroll y")
                + window().inner_height().unwrap().as_f64().unwrap();
            let height = document()
                .get_element_by_id("updates")
                .expect("should have updates")
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .offset_height() as f64;
            info!(
                "body scroll: {}, document scroll: {}",
                current_scroll, height
            );
            if current_scroll >= height {
                tmp_link.send_message(Msg::ScrolledDown);
            }
        }) as Box<dyn Fn()>);

        Updates {
            fetch_task: None,
            link,
            updates: vec![],
            token,
            is_fetching: false,
            closure,
            page: 1,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        window().set_onscroll(Some(self.closure.as_ref().unchecked_ref()));
        self.fetch_updates();
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::UpdatesReady(data) => {
                let mut updates = data.updates;
                self.updates.append(&mut updates);
                self.is_fetching = false;
            }
            Msg::ScrolledDown => {
                if !self.is_fetching {
                    self.page += 1;
                    self.fetch_updates();
                }
            }
            Noop => {
                return false;
            }
        };
        true
    }

    fn view(&self) -> Html {
        html! {
           <div class="container mx-auto pb-20" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <Spinner is_active=self.is_fetching />
                <div class="flex flex-col rounded-lg border border-grey-light m-2 shadow" id="updates">
                {
                for self.updates.iter().map(|update| html!{
                <RouterAnchor<AppRoute>
                classes="flex inline-flex border-b border-grey-light p-2 content-center"
                route=AppRoute::Chapter(update.source.clone(), base64::encode_config(update.title.clone(), base64::URL_SAFE_NO_PAD), update.number.clone(), 1)>
                    <div class="mr-4 my-2 h-16 w-16 object-fit object-center bg-center bg-cover rounded-full" style={format!("background-image: url({})", update.thumbnail_url.clone())}/>
                    <div class="flex flex-col my-auto">
                        <span class="text-lg font-semibold">{update.title.clone()}</span>
                        <span class="text-md">{format!("Chapter {}", update.number.clone())}</span>
                    </div>
                </RouterAnchor<AppRoute>>
                })
                }
                </div>
            </div>
        }
    }
    fn destroy(&mut self) {
        window().set_onscroll(None);
    }
}

impl Updates {
    fn fetch_updates(&mut self) {
        let req = Request::get(format!("/api/updates?page={}", self.page))
            .header("Authorization", self.token.to_string())
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<UpdatesResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::UpdatesReady(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
            self.is_fetching = true;
        }
    }
}
