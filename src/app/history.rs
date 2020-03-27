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

use crate::app::component::model::{HistoryModel, HistoryResponse};
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, Utc};
use serde_json::error::Category::Data;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Properties)]
pub struct Props {}

#[derive(Deserialize)]
pub struct Token {
    token: String,
}

pub struct History {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    history: Vec<HistoryModel>,
    token: String,
    is_fetching: bool,
    closure: Closure<dyn Fn()>,
    page: i32,
    prev_days: i64,
}

pub enum Msg {
    HistoryReady(HistoryResponse),
    ScrolledDown,
    noop,
}

impl Component for History {
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

            if current_scroll >= height {
                tmp_link.send_message(Msg::ScrolledDown);
            }
        }) as Box<dyn Fn()>);

        History {
            fetch_task: None,
            link,
            history: vec![],
            token,
            is_fetching: false,
            closure,
            page: 1,
            prev_days: -1,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        window().set_onscroll(Some(self.closure.as_ref().unchecked_ref()));
        self.fetch_updates();
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::HistoryReady(data) => {
                let mut history = data.history;
                if history.is_empty() {
                    window().set_onscroll(None);
                } else {
                    for his in history.iter_mut() {
                        let days = self.calculate_days(his.at);
                        if self.prev_days != days {
                            self.prev_days = days;
                            his.days = Some(days);
                            his.show_sep = Some(true);
                        }
                    }
                    self.history.append(&mut history);
                }
                self.is_fetching = false;
            }
            Msg::ScrolledDown => {
                if !self.is_fetching {
                    self.page += 1;
                    self.fetch_updates();
                }
            }
            noop => {
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
                for self.history.iter().map(|h| {
                html!{
                <>
                <div class={if h.show_sep.unwrap_or(false) {"shadow p-2 bg-teal-200"} else {"hidden"}}>
                <span class="text-semibold">{
                    match h.days.unwrap_or(0) {
                        0 => "Today".to_string(),
                        1 => "Yesterday".to_string(),
                        _ => format!("{} Days Ago", h.days.unwrap_or(0))
                    }
                }
                </span>
                </div>
                <RouterAnchor<AppRoute>
                classes="flex inline-flex border-b border-gray-light p-2 content-center hover:bg-gray-200"
                route=AppRoute::Chapter(h.source.clone(), base64::encode_config(h.title.clone(), base64::URL_SAFE_NO_PAD), h.chapter.clone(), 1)>
                    <div class="mr-4 my-2 h-16 w-16 object-fit object-center bg-center bg-cover rounded-full" style={format!("background-image: url({})", h.thumbnail_url.clone())}/>
                    <div class="flex flex-col my-auto">
                        <span class="text-lg font-semibold">{h.title.clone()}</span>
                        <span class="text-md">{format!("Chapter {}", h.chapter.clone())}</span>
                    </div>
                </RouterAnchor<AppRoute>>
                </>
                }
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

impl History {
    fn calculate_days(&self, at: chrono::DateTime<Local>) -> i64 {
        let timestamp = js_sys::Date::now();
        let secs: i64 = (timestamp / 1000.0).floor() as i64;
        let nanoes: u32 = (timestamp as u32 % 1000) * 1_000_000;
        let naivetime = chrono::NaiveDateTime::from_timestamp(secs, nanoes);
        let today = DateTime::<Utc>::from_utc(naivetime, Utc);
        let at = DateTime::<Utc>::from(at);
        today.signed_duration_since(at).num_days()
    }
    fn fetch_updates(&mut self) {
        let req = Request::get(format!("/api/history?page={}", self.page))
            .header("Authorization", self.token.to_string())
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<HistoryResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::HistoryReady(data);
                        }
                    }
                    Msg::noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
            self.is_fetching = true;
        }
    }
}
