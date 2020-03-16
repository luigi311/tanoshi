use anyhow;
use std::convert::TryInto;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::{agent::RouteRequest, prelude::*};

use crate::app::AppRoute;

use super::component::model::{
    ChapterModel, GetChaptersResponse, GetMangaResponse, GetPagesResponse, MangaModel,
};
use super::component::Spinner;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::HtmlElement;
use yew::prelude::*;
use yew::utils::document;

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
}

pub enum Msg {
    MangaReady(GetMangaResponse),
    ChapterReady(GetChaptersResponse),
    PagesReady(GetPagesResponse),
    PageForward,
    PagePrevious,
    RouterCallback,
    Noop,
}

impl Component for Chapter {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let callback = link.callback(|_| Msg::RouterCallback); // TODO use a dispatcher instead.
        let router = RouteAgent::bridge(callback);

        Chapter {
            fetch_task: None,
            link,
            router,
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
                self.is_fetching = false;
            }
            Msg::PageForward => {
                self.next_page_or_chapter();
            }
            Msg::PagePrevious => {
                self.prev_page_or_chapter();
            }
            Msg::RouterCallback => {
                self.get_pages();
            }
            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container mx-auto h-screen outline-none" id="manga-reader" tabindex="0" onkeydown=self.link.callback(|e: KeyboardEvent|
                match e.key().as_str() {
                    "ArrowRight" => Msg::PageForward,
                    "ArrowLeft"  => Msg::PagePrevious,
                    _ => Msg::Noop,
                }
            )>
                <RouterAnchor<AppRoute> classes="fixed left-0 top-0 z-50 ml-5 mt-5" route=AppRoute::Detail(self.source.to_owned(), self.title.to_owned())>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
                        <path class="heroicon-ui" d="M5.41 11H21a1 1 0 0 1 0 2H5.41l5.3 5.3a1 1 0 0 1-1.42 1.4l-7-7a1 1 0 0 1 0-1.4l7-7a1 1 0 0 1 1.42 1.4L5.4 11z"/>
                    </svg>
               </RouterAnchor<AppRoute>>
                <button class="manga-navigate-left outline-none" onmouseup=self.link.callback(|_| Msg::PagePrevious)/>
                <button class="manga-navigate-right outline-none" onmouseup=self.link.callback(|_| Msg::PageForward)/>
                <div class="flex">
                    {
                        for (0..self.pages.len()).map(|i| html! {
                        <img class={if (self.current_page == i) || (self.double_page && (self.current_page + 1 == i)) {
                            "manga-page active"
                        } else {
                            "manga-page"
                        }} src=self.pages[i] page={i}/>
                        })
                    }
                </div>
                <Spinner is_active=self.is_fetching />
            </div>
        }
    }
}

impl Chapter {
    fn get_chapters(&mut self) {
        let req = Request::get(format!(
            "/api/source/{}/manga/{}/chapter",
            self.source, self.title
        ))
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

    fn next_page_or_chapter(&mut self) {
        let mut num = 1;
        if self.double_page {
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
        if self.double_page {
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
}
