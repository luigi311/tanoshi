use stdweb::unstable::TryInto;
use stdweb::web::{document, HtmlElement, IHtmlElement, IParentNode};
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew_router::{agent::RouteRequest, prelude::*};

use super::{ChapterModel, MangaModel};

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
    current_chapter: String,
    chapter: ChapterModel,
    current_page: usize,
    double_page: bool,
    chapter_list: Vec<String>,
    previous_chapter_page: usize,
}

pub enum Msg {
    MangaReady(MangaModel),
    ChapterReady(ChapterModel),
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
            chapter_list: vec![],
            previous_chapter_page: 0,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.get_manga_info();
        let reader: HtmlElement = document().query_selector("#manga-reader")
            .unwrap()
            .expect("failed to get")
            .try_into()
            .unwrap();
        reader.focus();
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(data) => {
                self.chapter_list = data.chapters;
                self.get_chapter();
            }
            Msg::ChapterReady(data) => {
                self.chapter = data;
            }
            Msg::PageForward => {
                self.next_page_or_chapter();
            }
            Msg::PagePrevious => {
                self.prev_page_or_chapter();
            }
            Msg::RouterCallback => {
                self.get_chapter();
            }
            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container-fluid" id="manga-reader" tabindex="0" onkeydown=self.link.callback(|e: KeyDownEvent|
                match e.key().as_str() {
                    "ArrowRight" => Msg::PageForward,
                    "ArrowLeft"  => Msg::PagePrevious,
                    _ => Msg::Noop,
                }
            )>
                <button class="manga-navigate-left" onmouseup=self.link.callback(|_| Msg::PagePrevious)/>
                <button class="manga-navigate-right" onmouseup=self.link.callback(|_| Msg::PageForward)/>
                <div class="manga-page-container">
                    {
                        for (0..self.chapter.pages.len()).map(|i| html! {
                        <img class={if (self.current_page == i) || (self.double_page && (self.current_page + 1 == i)) {
                            "manga-page active"
                        } else {
                            "manga-page"
                        }} src=self.chapter.pages[i] page={i}/>
                        })
                    }
                </div>
            </div>
        }
    }
}

impl Chapter {
    fn get_manga_info(&mut self) {
        let req = Request::get(format!("/api/source/{}/manga/{}", self.source, self.title))
            .body(Nothing)
            .expect("failed to build request");

        let task = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Json<Result<MangaModel, failure::Error>>>| {
                if let (meta, Json(Ok(data))) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::MangaReady(data);
                    }
                }
                Msg::Noop
            }));
        self.fetch_task = Some(task);
    }

    fn get_chapter(&mut self) {
        let req = Request::get(format!("/api/source/{}/manga/{}/chapter/{}", self.source, self.title, self.current_chapter))
            .body(Nothing)
            .expect("failed to build request");

        let task = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Json<Result<ChapterModel, failure::Error>>>| {
                if let (meta, Json(Ok(data))) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::ChapterReady(data);
                    }
                }
                Msg::Noop
            }));
        self.fetch_task = Some(task);
    }

    fn next_page_or_chapter(&mut self) {
        let mut num = 1;
        if self.double_page {
            num = 2;
        }

        self.current_page += num;
        self.current_page = match self.chapter.pages.get(self.current_page) {
            Some(_) => self.current_page,
            None => 0,
        };

        let route_string: String;
        if self.current_page == 0 {
            let current_chapter_idx = match self.chapter_list.iter().position(|chapter| chapter == &self.current_chapter) {
                Some(index) => index,
                None => 0,
            };

            let is_next = match current_chapter_idx.checked_sub(1) {
                Some(index) => {
                    self.current_chapter = self.chapter_list[index].clone();
                    true
                }
                None => false,
            };


            self.chapter.pages.clear();

            if is_next {
                route_string = format!("/catalogue/{}/manga/{}/chapter/{}/page/1", self.source, self.title, self.current_chapter);
                self.previous_chapter_page = self.current_page;
            } else {
                route_string = format!("/catalogue/{}/manga/{}", self.source, self.title);
            }
            let route = Route::from(route_string);
            self.router.send(RouteRequest::ChangeRoute(route));
        } else {
            route_string = format!("/catalogue/{}/manga/{}/chapter/{}/page/{}", self.source, self.title, self.current_chapter, self.current_page + 1);
            let route = Route::from(route_string);
            self.router.send(RouteRequest::ReplaceRouteNoBroadcast(route));
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
            let current_chapter_idx = match self.chapter_list.iter().position(|chapter| chapter == &self.current_chapter) {
                Some(index) => index + 1,
                None => 0,
            };

            self.current_chapter = match self.chapter_list.get(current_chapter_idx) {
                Some(chapter) => chapter.clone(),
                None => self.current_chapter.clone(),
            };
            self.current_page = self.previous_chapter_page;
            if current_chapter_idx < self.chapter_list.len() {
                self.chapter.pages.clear();
                let route_string = format!("/catalogue/{}/manga/{}/chapter/{}/page/{}", self.source, self.title, self.current_chapter, self.current_page + 1);
                let route = Route::from(route_string);
                self.router.send(RouteRequest::ChangeRoute(route));
            } else {
                let route_string = format!("/catalogue/{}/manga/{}", self.source, self.title);
                let route = Route::from(route_string);
                self.router.send(RouteRequest::ChangeRoute(route));
            }
        } else {
            let route_string = format!("/catalogue/{}/manga/{}/chapter/{}/page/{}", self.source, self.title, self.current_chapter, self.current_page + 1);
            let route = Route::from(route_string);
            self.router.send(RouteRequest::ReplaceRouteNoBroadcast(route));
        }
    }
}
