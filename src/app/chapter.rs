use anyhow;
use chrono::{DateTime, Utc};
use js_sys;
use serde_json::json;
use wasm_bindgen::JsCast;
use web_sys::{CssStyleDeclaration, HtmlElement};
use yew::format::{Json, Nothing, Text};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::utils::{document, window};
use yew::{html, Component, ComponentLink, Html, InputData, Properties, ShouldRender};
use yew_router::{agent::RouteRequest, prelude::*};

use crate::app::component::model::{HistoryRequest, HistoryResponse, SettingParams};
use crate::app::{browse::BrowseRoute, AppRoute};

use super::component::model::{
    BackgroundColor, ChapterModel, GetChaptersResponse, GetMangaResponse, GetPagesResponse,
    MangaModel, PageRendering, ReadingDirection,
};
use super::component::Spinner;
use wasm_bindgen::__rt::std::net::Shutdown::Read;

#[derive(Clone, Properties)]
pub struct Props {
    pub source: String,
    pub title: String,
    pub chapter: String,
    pub page: usize,
}

pub struct Chapter {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    router: Box<dyn Bridge<RouteAgent>>,
    token: String,
    source: String,
    title: String,
    chapter: ChapterModel,
    current_chapter: String,
    current_page: usize,
    double_page: bool,
    chapters: Vec<ChapterModel>,
    previous_chapter_page: usize,
    pages: Vec<String>,
    is_fetching: bool,
    refs: Vec<NodeRef>,
    is_bar_visible: bool,
    settings: SettingParams,
    page_refs: Vec<NodeRef>,
}

pub enum Msg {
    MangaReady(GetMangaResponse),
    ChapterReady(GetChaptersResponse),
    PagesReady(GetPagesResponse),
    PageForward,
    PagePrevious,
    ToggleBar,
    PageSliderChange(usize),
    RouterCallback,
    SetHistoryRequested,
    noop,
}

impl Component for Chapter {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let callback = link.callback(|_| Msg::RouterCallback);
        let router = RouteAgent::bridge(callback);
        let storage = StorageService::new(Area::Local).unwrap();
        let settings = {
            if let Ok(settings) = storage.restore("settings") {
                serde_json::from_str(settings.as_str()).expect("failed to serialize")
            } else {
                SettingParams::default()
            }
        };
        let token = {
            if let Ok(token) = storage.restore("token") {
                token
            } else {
                "".to_string()
            }
        };

        if settings.background_color == BackgroundColor::Black {
            document()
                .body()
                .expect("document should have a body")
                .dyn_ref::<web_sys::HtmlElement>()
                .unwrap()
                .style()
                .set_property("background-color", "black");
        }

        Chapter {
            fetch_task: None,
            link,
            router,
            token,
            source: props.source,
            title: props.title,
            current_chapter: props.chapter,
            chapter: Default::default(),
            current_page: props.page - 1,
            double_page: false,
            chapters: vec![],
            previous_chapter_page: 0,
            pages: vec![],
            is_fetching: false,
            refs: vec![NodeRef::default(), NodeRef::default()],
            is_bar_visible: true,
            settings,
            page_refs: vec![],
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.get_chapters();
        document()
            .get_element_by_id("manga-reader")
            .expect("should have manga reader")
            .dyn_ref::<HtmlElement>()
            .expect("should load HtmlElement")
            .focus();
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(data) => {}
            Msg::ChapterReady(data) => {
                self.is_fetching = false;
                self.chapters = data.chapters.clone();
                let idx = match self
                    .chapters
                    .iter()
                    .position(|chapter| chapter.no == self.current_chapter)
                {
                    Some(index) => index,
                    None => 0,
                };
                self.chapter = data.chapters[idx].clone();
                self.get_pages();
            }
            Msg::PagesReady(data) => {
                self.pages = data.pages;
                self.page_refs.clear();
                self.page_refs.resize(self.pages.len(), NodeRef::default());
                self.is_fetching = false;
            }
            Msg::PageForward => {
                if self.settings.reading_direction == ReadingDirection::LeftToRight {
                    self.next_page_or_chapter();
                } else {
                    self.prev_page_or_chapter();
                }
                self.set_history();
            }
            Msg::PagePrevious => {
                if self.settings.reading_direction == ReadingDirection::LeftToRight {
                    self.prev_page_or_chapter();
                } else {
                    self.next_page_or_chapter();
                }
                self.set_history();
            }
            Msg::PageSliderChange(page) => {
                self.move_to_page(page);
                self.set_history();
            }
            Msg::ToggleBar => {
                if self.is_bar_visible {
                    if let Some(bar) = self.refs[0].cast::<HtmlElement>() {
                        bar.class_list().remove_1("slideInDown");
                        bar.class_list().add_1("slideOutUp");
                        self.is_bar_visible = false;
                    }
                    if let Some(bar) = self.refs[1].cast::<HtmlElement>() {
                        bar.class_list().remove_1("slideInUp");
                        bar.class_list().add_1("slideOutDown");
                        self.is_bar_visible = false;
                    }
                } else {
                    if let Some(bar) = self.refs[0].cast::<HtmlElement>() {
                        bar.class_list().remove_1("slideOutUp");
                        bar.class_list().add_1("slideInDown");
                        self.is_bar_visible = true;
                    }
                    if let Some(bar) = self.refs[1].cast::<HtmlElement>() {
                        bar.class_list().remove_1("slideOutDown");
                        bar.class_list().add_1("slideInUp");
                        self.is_bar_visible = true;
                    }
                }
            }
            Msg::RouterCallback => {
                self.get_pages();
            }
            Msg::SetHistoryRequested => {}
            Msg::noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
        <div >
            <div
            ref=self.refs[0].clone()
            class="animated slideInDown faster block fixed inset-x-0 top-0 z-50 bg-gray-900 z-50 content-end flex opacity-75"
            style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <RouterAnchor<AppRoute> classes="z-50 ml-2 mb-2 text-white" route=AppRoute::Browse(BrowseRoute::Detail(self.source.to_owned(), self.title.to_owned()))>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" class="fill-current inline-block mb-1">
                        <path class="heroicon-ui" d="M5.41 11H21a1 1 0 0 1 0 2H5.41l5.3 5.3a1 1 0 0 1-1.42 1.4l-7-7a1 1 0 0 1 0-1.4l7-7a1 1 0 0 1 1.42 1.4L5.4 11z"/>
                    </svg>
               </RouterAnchor<AppRoute>>
            </div>
            <div class="h-screen m-0 outline-none" id="manga-reader" tabindex="0" onkeydown=self.link.callback(|e: KeyboardEvent|
                match e.key().as_str() {
                    "ArrowRight" => Msg::PageForward,
                    "ArrowLeft"  => Msg::PagePrevious,
                    _ => Msg::noop,
                }
            )>
                <button
                class={
                    if self.settings.page_rendering == PageRendering::LongStrip {
                        "hidden"
                    } else {
                        "manga-navigate-left outline-none fixed"
                    }
                }
                onmouseup=self.link.callback(|_| Msg::PagePrevious)/>
                <button class="manga-navigate-center outline-none fixed" onmouseup=self.link.callback(|_| Msg::ToggleBar)/>
                <button
                class={
                    if self.settings.page_rendering == PageRendering::LongStrip {
                        "hidden"
                    } else {
                        "manga-navigate-right outline-none fixed"
                    }
                }
                onmouseup=self.link.callback(|_| Msg::PageForward)/>
                <div class={
                    format!("flex justify-center {} {}",
                    if self.settings.page_rendering == PageRendering::LongStrip {"flex-col cursor-pointer"} else {"h-screen"},
                    if self.settings.reading_direction == ReadingDirection::RightToLeft {"flex-row-reverse"} else {""})
                }
                onmouseup={
                    if self.settings.page_rendering == PageRendering::LongStrip {
                    self.link.callback(|_| Msg::ToggleBar)
                    } else {
                    self.link.callback(|_| Msg::noop)
                    }
                }>
                    {
                        for (0..self.pages.len()).map(|i| html! {
                        <img ref=self.page_refs[i].clone() class={format!("{} {}", if self.settings.page_rendering == PageRendering::DoublePage {
                            "h-screen"
                        } else {
                            "w-auto h-auto object-contain"
                        }, if (self.current_page == i)
                        || (self.settings.page_rendering == PageRendering::DoublePage && (self.current_page + 1 == i)
                        || self.settings.page_rendering == PageRendering::LongStrip) {
                            "block"
                        } else {
                            "hidden"
                        })} src=self.pages[i] page={i}/>
                        })
                    }
                </div>
                <Spinner is_active=self.is_fetching />
            </div>
            <div ref=self.refs[1].clone()
            class="animated slideInUp faster block fixed inset-x-0 bottom-0 z-50 bg-gray-900 opacity-75 shadow safe-bottom">
                <div class="flex px-4 py-5 justify-center">
                    <input type="range" min="0" max={if self.pages.len() > 0 {self.pages.len()-1} else {self.pages.len()}} step="1" value={self.current_page} defaultValue={self.current_page} oninput=self.link.callback(|e: InputData| Msg::PageSliderChange(e.value.parse::<usize>().unwrap()))/>
                    <span class="mx-4 text-white">{format!("{}/{}", self.current_page + 1, self.pages.len())}</span>
                </div>
            </div>
        </div>
        }
    }

    fn destroy(&mut self) {
        if self.settings.background_color == BackgroundColor::Black {
            document()
                .body()
                .expect("document should have a body")
                .dyn_ref::<web_sys::HtmlElement>()
                .unwrap()
                .style()
                .set_property("background-color", "white");
        }
    }
}

impl Chapter {
    fn get_chapters(&mut self) {
        let req = Request::get(format!(
            "/api/source/{}/manga/{}/chapter",
            self.source, self.title
        ))
        .header("Authorization", self.token.to_string())
        .body(Nothing)
        .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetChaptersResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::ChapterReady(data);
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

    fn get_pages(&mut self) {
        let req = Request::get(format!(
            "/api/source/{}/manga/{}/chapter/{}",
            self.source, self.title, self.current_chapter
        ))
        .body(Nothing)
        .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetPagesResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::PagesReady(data);
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

    fn move_to_page(&mut self, page: usize) {
        self.current_page = page;
        let route_string = format!(
            "/catalogue/{}/manga/{}/chapter/{}/page/{}",
            self.source, self.title, self.current_chapter, self.current_page
        );
        let route = Route::from(route_string);
        self.router
            .send(RouteRequest::ReplaceRouteNoBroadcast(route));
    }

    fn next_page_or_chapter(&mut self) {
        let mut num = 1;
        if self.settings.page_rendering == PageRendering::DoublePage {
            num = 2;
        }

        self.current_page += num;
        self.current_page = match self.pages.get(self.current_page) {
            Some(_) => self.current_page,
            None => 0,
        };

        let route_string: String;
        if self.current_page == 0 {
            let current_chapter_idx = match self
                .chapters
                .iter()
                .position(|chapter| chapter.no == self.current_chapter)
            {
                Some(index) => index,
                None => 0,
            };

            let is_next = match current_chapter_idx.checked_sub(1) {
                Some(index) => {
                    self.current_chapter = self.chapters[index].no.clone();
                    true
                }
                None => false,
            };

            self.pages.clear();

            if is_next {
                route_string = format!(
                    "/catalogue/{}/manga/{}/chapter/{}/page/1",
                    self.source, self.title, self.current_chapter
                );
                self.previous_chapter_page = self.current_page;
            } else {
                route_string = format!("/catalogue/{}/manga/{}", self.source, self.title);
            }
            let route = Route::from(route_string);
            self.router.send(RouteRequest::ChangeRoute(route));
        } else {
            route_string = format!(
                "/catalogue/{}/manga/{}/chapter/{}/page/{}",
                self.source,
                self.title,
                self.current_chapter,
                self.current_page + 1
            );
            let route = Route::from(route_string);
            self.router
                .send(RouteRequest::ReplaceRouteNoBroadcast(route));
        }
    }

    fn prev_page_or_chapter(&mut self) {
        let mut num: usize = 1;
        if self.settings.page_rendering == PageRendering::DoublePage {
            num = 2;
        }

        let is_prev = match self.current_page.checked_sub(num) {
            Some(page) => {
                self.current_page = page;
                false
            }
            None => true,
        };

        if is_prev {
            let current_chapter_idx = match self
                .chapters
                .iter()
                .position(|chapter| chapter.no == self.current_chapter)
            {
                Some(index) => index + 1,
                None => 0,
            };

            self.current_chapter = match self.chapters.get(current_chapter_idx) {
                Some(chapter) => chapter.no.to_string(),
                None => self.current_chapter.clone(),
            };
            self.current_page = self.previous_chapter_page;
            if current_chapter_idx < self.chapters.len() {
                self.pages.clear();
                let route_string = format!(
                    "/catalogue/{}/manga/{}/chapter/{}/page/{}",
                    self.source,
                    self.title,
                    self.current_chapter,
                    self.current_page + 1
                );
                let route = Route::from(route_string);
                self.router.send(RouteRequest::ChangeRoute(route));
            } else {
                let route_string = format!("/catalogue/{}/manga/{}", self.source, self.title);
                let route = Route::from(route_string);
                self.router.send(RouteRequest::ChangeRoute(route));
            }
        } else {
            let route_string = format!(
                "/catalogue/{}/manga/{}/chapter/{}/page/{}",
                self.source,
                self.title,
                self.current_chapter,
                self.current_page + 1
            );
            let route = Route::from(route_string);
            self.router
                .send(RouteRequest::ReplaceRouteNoBroadcast(route));
        }
    }

    fn get_date(&self) -> DateTime<Utc> {
        let timestamp = js_sys::Date::now();
        let secs: i64 = (timestamp / 1000.0).floor() as i64;
        let nanoes: u32 = (timestamp as u32 % 1000) * 1_000_000;
        let naivetime = chrono::NaiveDateTime::from_timestamp(secs, nanoes);
        DateTime::<Utc>::from_utc(naivetime, Utc)
    }

    fn set_history(&mut self) {
        let h = HistoryRequest {
            source: self.source.clone(),
            title: String::from_utf8(
                base64::decode_config(self.title.clone(), base64::URL_SAFE_NO_PAD).unwrap(),
            )
            .unwrap(),
            chapter: self.current_chapter.clone(),
            read: self.current_page as i32,
            at: DateTime::from(self.get_date()),
        };

        let req = Request::post("/api/history")
            .header("Authorization", self.token.to_string())
            .header("Content-Type", "application/json")
            .body(Json(&h))
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Text>| {
                if let (meta, Ok(data)) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::SetHistoryRequested;
                    }
                }
                Msg::noop
            }),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
}
