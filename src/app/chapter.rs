use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use stdweb::unstable::TryInto;
use stdweb::web::{document, Document, IParentNode};
use stdweb::web::HtmlElement;
use stdweb::web::IHtmlElement;
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew::format::{Json, Nothing, Text};
use yew::html::{ChildrenRenderer, NodeRef};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew_router::components::RouterAnchor;

use crate::app::AppRoute;

use super::{ChapterModel, MangaModel};
use super::component::{Manga, TopBar};

#[derive(Clone, Properties)]
pub struct Props {
    pub source: String,
    pub title: String,
    pub chapter: String,
}

pub struct Chapter {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    source: String,
    title: String,
    current_chapter: String,
    chapter: ChapterModel,
    current_page: usize,
    double_page: bool,
    chapter_list: Vec<String>,
}

pub enum Msg {
    MangaReady(MangaModel),
    ChapterReady(ChapterModel),
    PageForward,
    PagePrevious,
    Noop,
}

impl Component for Chapter {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Chapter {
            fetch_task: None,
            link,
            source: props.source,
            title: props.title,
            current_chapter: props.chapter,
            chapter: Default::default(),
            current_page: 0,
            double_page: true,
            chapter_list: vec![],
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.get_chapter();
        let reader: HtmlElement = document().query_selector(".manga-reader-container")
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
            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <>
            <TopBar />
            <div class="manga-reader-container" tabindex="0" onkeydown=self.link.callback(|e: KeyDownEvent|
                match e.key().as_str() {
                    "ArrowRight" => Msg::PageForward,
                    "ArrowLeft"  => Msg::PagePrevious,
                    _ => Msg::Noop,
                }
            )>
                <button class="manga-navigate-left" onclick=self.link.callback(|_| Msg::PagePrevious)/>
                <button class="manga-navigate-right" onclick=self.link.callback(|_| Msg::PageForward)/>
                <div class="manga-page-container">
                    {
                        if self.chapter.pages.len() > 0 {
                            if self.double_page {
                                html! {
                                    <>
                                        {
                                            match self.chapter.pages.get(self.current_page+1) {
                                                Some(page) => html! { <img class="manga-page" src=page/> },
                                                None => html!{},
                                            }
                                        }

                                        <img class="manga-page" src=self.chapter.pages[self.current_page]/>
                                    </>
                                }
                            } else {
                                html! {
                                <>
                                    <img class="manga-page" src=self.chapter.pages[self.current_page]/>
                                </>
                                }
                            }
                        } else {
                            html! {
                                <>
                                </>
                            }
                        }
                    }
                </div>
            </div>
            </>
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

        if self.current_page == 0 {
            let next_chapter = match self.chapter_list.iter().position(|chapter| chapter == &self.current_chapter) {
                Some(index) => index + 1,
                None => 0,
            };

            self.current_chapter = match self.chapter_list.get(next_chapter) {
                Some(chapter) => chapter.to_owned(),
                None => self.current_chapter.to_owned(),
            };

            self.get_chapter();
        }
    }

    fn prev_page_or_chapter(&mut self) {
        let mut num: usize = 1;
        if self.double_page {
            num = 2;
        }
        self.current_page = match self.current_page.checked_sub(num) {
            Some(page) => page,
            None => 0
        }
    }
}
