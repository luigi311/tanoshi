use anyhow;
use chrono::{DateTime, Utc};
use js_sys;
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CssStyleDeclaration, HtmlElement, HtmlImageElement};
use yew::{Component, ComponentLink, html, Html, InputData, Properties, ShouldRender};
use yew::format::{Json, Nothing, Text};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::utils::{document, window};
use yew_router::{agent::RouteRequest, prelude::*};

use crate::app::{AppRoute, browse::BrowseRoute, job};
use crate::app::component::model::{HistoryRequest, HistoryResponse, SettingParams};

use super::component::model::{
    BackgroundColor, ChapterModel, GetChaptersResponse, GetMangaResponse, GetPagesResponse,
    MangaModel, PageRendering, ReadingDirection,
};
use super::component::Spinner;

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
    chapters: Vec<ChapterModel>,
    previous_chapter_page: usize,
    pages: Vec<String>,
    is_fetching: bool,
    refs: Vec<NodeRef>,
    is_bar_visible: bool,
    settings: SettingParams,
    page_refs: Vec<NodeRef>,
    container_ref: NodeRef,
    closure: Closure<dyn Fn()>,
    is_history_fetching: bool,
    worker: Box<dyn Bridge<job::Worker>>,
    should_fetch: bool,
}

pub enum Msg {
    ChapterReady(GetChaptersResponse),
    PagesReady(GetPagesResponse),
    PageForward,
    PagePrevious,
    ToggleBar,
    PageSliderChange(usize),
    RouterCallback,
    SetHistoryRequested,
    ScrollEvent(f64),
    Noop,
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
                .set_property("background-color", "black")
                .expect("failed to set background color");
        }

        let tmp_link = link.clone();
        let closure = Closure::wrap(Box::new(move || {
            let current_scroll = window().scroll_y().expect("error get scroll y")
                + window().inner_height().unwrap().as_f64().unwrap();
            let height = document()
                .get_element_by_id("pages")
                .expect("should have pages")
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .offset_height() as f64;

            tmp_link.send_message(Msg::ScrollEvent(current_scroll));
        }) as Box<dyn Fn()>);

        let worker_callback = link.callback(|_| Msg::SetHistoryRequested);
        let worker = job::Worker::bridge(worker_callback);

        Chapter {
            fetch_task: None,
            link,
            router,
            token,
            source: props.source,
            title: props.title,
            current_chapter: props.chapter,
            chapter: Default::default(),
            current_page: props.page.checked_sub(1).unwrap_or(0),
            chapters: vec![],
            previous_chapter_page: 0,
            pages: vec![],
            is_fetching: false,
            refs: vec![NodeRef::default(), NodeRef::default()],
            is_bar_visible: true,
            settings,
            page_refs: vec![],
            container_ref: NodeRef::default(),
            closure,
            is_history_fetching: false,
            worker,
            should_fetch: true,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.source != props.source || self.title != props.title || self.current_chapter != props.chapter || self.current_page != props.page.checked_sub(1).unwrap_or(0) {
            self.source = props.source;
            self.title = props.title;
            self.current_chapter = props.chapter;
            self.current_page = props.page.checked_sub(1).unwrap_or(0);
            return true;
        }
        false
    }

    fn rendered(&mut self, first_render: bool) {
        if self.should_fetch {
            self.get_chapters();
            self.should_fetch = false;
        }
        document()
            .get_element_by_id("manga-reader")
            .expect("should have manga reader")
            .dyn_ref::<HtmlElement>()
            .expect("should load HtmlElement")
            .focus()
            .unwrap();
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
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
                return false;
            }
            Msg::PagesReady(data) => {
                self.pages = data.pages;
                self.page_refs.clear();
                for i in 0..self.pages.len() + 1 {
                    self.page_refs.push(NodeRef::default());
                }

                if self.settings.page_rendering == PageRendering::LongStrip {
                    match window().onscroll() {
                        Some(_) => {}
                        None => window().set_onscroll(Some(self.closure.as_ref().unchecked_ref())),
                    };
                }

                self.is_fetching = false;
            }
            Msg::PageForward => {
                if self.settings.page_rendering == PageRendering::LongStrip {
                    self.next_page_or_chapter();
                } else {
                    if self.settings.reading_direction == ReadingDirection::LeftToRight {
                        self.next_page_or_chapter();
                    } else {
                        self.prev_page_or_chapter();
                    }
                }
                self.set_history();
            }
            Msg::PagePrevious => {
                if self.settings.page_rendering == PageRendering::LongStrip {
                    self.prev_page_or_chapter();
                } else {
                    if self.settings.reading_direction == ReadingDirection::LeftToRight {
                        self.prev_page_or_chapter();
                    } else {
                        self.next_page_or_chapter();
                    }
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
            Msg::SetHistoryRequested => {
                self.is_history_fetching = false;
                return false;
            }
            Msg::ScrollEvent(scroll) => {
                let mut page = 0;
                for page_ref in self.page_refs.clone().iter() {
                    if let Some(el) = page_ref.cast::<HtmlImageElement>() {
                        if scroll > el.offset_top() as f64 {
                            page = el.id().parse::<usize>().unwrap();
                            if page == (self.pages.len().checked_sub(1).unwrap_or(0))
                                && page != self.current_page
                            {
                                self.current_page = page;
                                self.set_history();
                            }
                        } else {
                            if self.current_page != page {
                                self.current_page = page;
                                self.set_history();
                            }
                            break;
                        }
                    }
                }
            }
            Msg::Noop => {
                return false;
            }
        }
        true
    }

    fn view(&self) -> Html {
        return html! {
        <div>
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
                    _ => Msg::Noop,
                }
            )>
                {
                    if self.settings.page_rendering != PageRendering::LongStrip {
                        html!{
                            <>
                                <button class="manga-navigate-left outline-none fixed" onmouseup=self.link.callback(|_| Msg::PagePrevious)/>
                                <button class="manga-navigate-center outline-none fixed" onmouseup=self.link.callback(|_| Msg::ToggleBar)/>
                                <button class="manga-navigate-right outline-none fixed" onmouseup=self.link.callback(|_| Msg::PageForward)/>
                            </>
                        }
                    } else {
                        html!{}
                    }
                }
                <div ref=self.container_ref.clone()
                 id="pages"
                 class={
                    format!("flex justify-center overflow-auto {} {}",
                    if self.settings.page_rendering == PageRendering::LongStrip {"flex-col"} else {"h-screen"},
                    if self.settings.reading_direction == ReadingDirection::RightToLeft {"flex-row-reverse"} else {""})
                }>
                    {
                        if self.settings.page_rendering == PageRendering::LongStrip {
                            html!{
                            <div
                                class="border-dashed border-b border-gray-500 flex justify-center items-center h-24 cursor-pointer"
                                onmouseup=self.link.callback(|_| Msg::PagePrevious)>
                                <span class="text-gray-500">{"Previous Chapter"}</span>
                            </div>
                            }
                        } else {
                            html!{}
                        }
                    }
                    {
                        match self.settings.page_rendering {
                            PageRendering::DoublePage => self.double_page_view(),
                            PageRendering::SinglePage => self.single_page_view(),
                            PageRendering::LongStrip => self.long_strip_view(),
                        }
                    }
                     {
                        if self.settings.page_rendering == PageRendering::LongStrip {
                            html!{
                            <div
                                class="border-dashed border-t border-gray-500 flex justify-center items-center h-24 cursor-pointer"
                                onmouseup=self.link.callback(|_| Msg::PageForward)>
                                <span class="text-gray-500">{"Next Chapter"}</span>
                            </div>
                            }
                        } else {
                            html!{}
                        }
                    }
                </div>
                <Spinner is_active=self.is_fetching is_fullscreen=true/>
            </div>
            <div ref=self.refs[1].clone()
            class="animated slideInUp faster block fixed inset-x-0 bottom-0 z-50 bg-gray-900 opacity-75 shadow safe-bottom">
                <div class="flex px-4 py-5 justify-center">
                    <input
                        dir={if self.settings.reading_direction == ReadingDirection::RightToLeft {"rtl"} else {"ltr"}}
                        type="range"
                        min="0"
                        max=self.pages.len().checked_sub(1).unwrap_or(0)
                        step="1"
                        value={self.current_page}
                        oninput=self.link.callback(|e: InputData| Msg::PageSliderChange(e.value.parse::<usize>().unwrap()))/>
                    <span class="mx-4 text-white">{format!("{}/{}", self.current_page + 1, self.pages.len())}</span>
                </div>
            </div>
        </div>
        };
    }

    fn destroy(&mut self) {
        if self.settings.page_rendering == PageRendering::LongStrip {
            window().set_onscroll(None);
        }
        if self.settings.background_color == BackgroundColor::Black {
            document()
                .body()
                .expect("document should have a body")
                .dyn_ref::<web_sys::HtmlElement>()
                .unwrap()
                .style()
                .set_property("background-color", "white")
                .expect("failed to set background color");
        }
    }
}

impl Chapter {
    fn single_page_view(&self) -> Html {
        let mut i = 0;
        self.pages.clone().into_iter().map(|page| html! {
            <img id={let temp = i; i += 1; temp}
                ref=self.page_refs[i].clone()
                class={format!("w-auto h-auto object-contain {}", if self.current_page == i {"block"} else {"hidden"})}
                src={if i >= 0 && i < self.current_page + 3 {page} else {"".to_string()}}
                onmouseup={self.link.callback(|_| Msg::Noop)}
            />
        }).collect()
    }
    fn double_page_view(&self) -> Html {
        let mut pages = Vec::new();

        let default_str = "".to_string();
        for i in 0..self.pages.len() {
            if i % 2 == 0 {
                pages.push((
                    match self.pages.get(i) {
                        Some(page) => page.clone(),
                        None => "".to_string()
                    },
                    match self.pages.get(i + 1) {
                        Some(page) => page.clone(),
                        None => "".to_string()
                    }));
            }
        }

        let mut i = 0;
        pages.into_iter().map(|(left_page, right_page)| html! {
        <>
            <img id={let temp = i; i += 1; temp}
                ref=self.page_refs[i].clone()
                class={
                    format!("w-1/2 h-auto object-contain {} {}",
                        if self.settings.reading_direction == ReadingDirection::RightToLeft {"object-left"} else {"object-right"},
                        if self.current_page == i {"block"} else {"hidden"})
                }
                src={if i >= 0 && i < self.current_page + 3 {left_page} else {"".to_string()}}
                onmouseup={self.link.callback(|_| Msg::Noop)}
            />
            <img id={let temp = i; i += 1; temp}
                ref=self.page_refs[i].clone()
                class={
                    format!("w-1/2 h-auto object-contain {} {}",
                        if self.settings.reading_direction == ReadingDirection::RightToLeft {"object-right"} else {"object-left"},
                        if self.current_page + 1 == i {"block"} else {"hidden"})
                }
                src={if i >= 0 && i < self.current_page + 3 {right_page} else {"".to_string()}}
                onmouseup={self.link.callback(|_| Msg::Noop)}
            />
        </>
        }).collect()
    }

    fn long_strip_view(&self) -> Html {
        let mut i = 0;
        self.pages.clone().into_iter().map(|page| html! {
            <img id={let temp = i; i += 1; temp}
                ref=self.page_refs[i].clone()
                class={format!("w-auto h-auto object-contain block")}
                src={page}
                onmouseup={self.link.callback(|_| Msg::ToggleBar)}
            />
        }).collect()
    }

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
                    Msg::Noop
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
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
            self.is_fetching = true;
        }
    }

    fn move_to_page(&mut self, page: usize) {
        self.current_page = page;
        if self.settings.page_rendering == PageRendering::LongStrip {
            if let Some(el) = self.page_refs[page].cast::<HtmlImageElement>() {
                el.scroll_into_view();
            }
        } else {}
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

                let route = Route::from(route_string);
                self.router.send(RouteRequest::ChangeRoute(route));
            }
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

    fn get_date(&self) -> chrono::NaiveDateTime {
        let timestamp = js_sys::Date::now();
        let secs: i64 = (timestamp / 1000.0).floor() as i64;
        let nanoes: u32 = (timestamp as u32 % 1000) * 1_000_000;
        chrono::NaiveDateTime::from_timestamp(secs, nanoes)
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
            at: self.get_date(),
        };
        self.worker
            .send(job::Request::PostHistory(self.token.clone(), h));
    }
}
