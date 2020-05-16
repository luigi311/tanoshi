use crate::app::AppRoute;
use serde::Deserialize;
use web_sys::HtmlElement;
use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::{FetchService, StorageService};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use yew_router::components::RouterAnchor;
use yew_router::service::RouteService;

use tanoshi::manga::{History as HistoryModel, HistoryResponse, Update as UpdateModel, UpdatesResponse};
use super::component::{Spinner};
use yew::utils::{document, window};

use crate::app::component::model::{};
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, Utc};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Debug, Eq, PartialEq)]
pub enum PageType {
    History,
    Updates
}

impl Into<PageType> for String {
    fn into(self) -> PageType {
        match self.as_str() {
            "/updates" => PageType::Updates,
            "/history" => PageType::History,
            _ => PageType::Updates,
        }
    }
}

#[derive(Clone, Properties)]
pub struct Props {

}

pub struct History {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    history: Vec<HistoryModel>,
    updates: Vec<UpdateModel>,
    token: String,
    is_fetching: bool,
    closure: Closure<dyn Fn()>,
    page: i32,
    prev_days: i64,
    should_fetch: bool,
    page_type: PageType,
    route_service: RouteService<()>,
}

pub enum Msg {
    HistoryReady(HistoryResponse),
    UpdatesReady(UpdatesResponse),
    ScrolledDown,
    Noop,
}

impl Component for History {
    type Message = Msg;
    type Properties = Props;

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
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

        let route_service: RouteService<()> = RouteService::new();
        let page_type : PageType = route_service.get_path().into();
        info!("create {:?}", page_type);

        History {
            fetch_task: None,
            link,
            history: vec![],
            updates: vec![],
            token,
            is_fetching: false,
            closure,
            page: 1,
            prev_days: -1,
            should_fetch: true,
            page_type,
            route_service,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        let page_type : PageType = self.route_service.get_path().into();
        if self.page_type != page_type {
            self.page_type = page_type;
            self.should_fetch = true;
            true
        } else {
            false
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if self.should_fetch {
            window().set_onscroll(Some(self.closure.as_ref().unchecked_ref()));
            match self.page_type {
                PageType::History => self.fetch_history(),
                PageType::Updates => self.fetch_updates(),
            }
            self.should_fetch = false;
        }
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
                    match self.page_type {
                        PageType::History => self.fetch_history(),
                        PageType::Updates => self.fetch_updates(),
                    }
                }
            }
            Msg::UpdatesReady(data) => {
                let mut updates = data.updates;
                for update in updates.iter_mut() {
                    let days = self.calculate_days(update.uploaded);
                    if self.prev_days != days {
                        self.prev_days = days;
                        update.days = Some(days);
                        update.show_sep = Some(true);
                    }
                }
                self.updates.append(&mut updates);
                self.is_fetching = false;
            }
            Msg::Noop => {
                return false;
            }
        };
        true
    }

    fn view(&self) -> Html {
        html! {
           <div class="container mx-auto pb-20" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <div class="flex flex-col rounded-lg border border-grey-light m-2 shadow" id="updates">
                {self.updates_or_history_cards()}
                </div>
                {
                    match self.is_fetching {
                        false => html!{
                            <div class="flex rounded-lg border border-grey-light m-2 shadow justify-center h-5">
                                <button class="w-full h-full block" onclick=self.link.callback(|_| Msg::ScrolledDown)>{"Load More"}</button>
                            </div>
                        },
                        true => html!{<Spinner is_active=self.is_fetching is_fullscreen=false />}
                    }
                }
            </div>
        }
    }
    fn destroy(&mut self) {
        window().set_onscroll(None);
    }
}

impl History {
    fn calculate_days(&self, at: chrono::NaiveDateTime) -> i64 {
        let timestamp = js_sys::Date::now();
        let secs: i64 = (timestamp / 1000.0).floor() as i64;
        let nanoes: u32 = (timestamp as u32 % 1000) * 1_000_000;
        let today = chrono::NaiveDateTime::from_timestamp(secs, nanoes);
        today.signed_duration_since(at).num_days()
    }
    
    fn show_separator(&self, show_sep: Option<bool>, days: Option<i64>) -> Html{
        html!{
            <div class={if show_sep.unwrap_or(false) {"shadow p-2 bg-tachiyomi-blue"} else {"hidden"}}>
                <span class="text-semibold text-white">{
                    match days.unwrap_or(0) {
                        0 => "Today".to_string(),
                        1 => "Yesterday".to_string(),
                        _ => format!("{} Days Ago", days.unwrap_or(0))
                    }
                }
                </span>
            </div>
        }
    }

    fn updates_or_history_cards(&self) -> Html {
        match self.page_type {
            PageType::History => {
                self.history.iter().map(|h| {
                    html!{
                        <>
                            {self.show_separator(h.show_sep, h.days)}
                            <RouterAnchor<AppRoute>
                            classes="flex inline-flex border-b border-gray-light p-2 content-center hover:bg-gray-200"
                            route=AppRoute::Chapter(h.manga_id, h.chapter_id, (h.read + 1) as usize)>
                                <div class="mr-4 my-2 h-16 w-16 object-fit object-center bg-center bg-cover rounded-full" style={format!("background-image: url({})", h.thumbnail_url.clone().unwrap_or("".to_string()))}/>
                                <div class="flex flex-col my-auto">
                                    <span class="text-lg font-semibold">{h.title.clone()}</span>
                                    <span class="text-md">{format!("Chapter {}", h.chapter.clone())}</span>
                                </div>
                            </RouterAnchor<AppRoute>>
                        </>
                    }
                }).collect()
            },
            PageType::Updates => {
                self.updates.iter().map(|update| {
                    html!{
                        <>
                            {self.show_separator(update.show_sep, update.days)}
                            <RouterAnchor<AppRoute>
                            classes="flex inline-flex border-b border-gray-light p-2 content-center hover:bg-gray-200"
                            route=AppRoute::Chapter(update.manga_id, update.chapter_id, 1)>
                                <div class="mr-4 my-2 h-16 w-16 object-fit object-center bg-center bg-cover rounded-full" style={format!("background-image: url({})", update.thumbnail_url.clone())}/>
                                <div class="flex flex-col my-auto">
                                    <span class="text-lg font-semibold">{update.title.clone()}</span>
                                    <span class="text-md">{format!("Chapter {}", update.number.clone())}</span>
                                </div>
                            </RouterAnchor<AppRoute>>
                        </>
                    }
                }).collect()
            }
        }
    }

    fn fetch_history(&mut self) {
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
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
            self.is_fetching = true;
        }
    }

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
