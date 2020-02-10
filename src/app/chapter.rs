use serde::{Deserialize, Serialize};
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew::format::{Json, Nothing, Text};
use yew::html::{ChildrenRenderer, NodeRef};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use std::collections::HashMap;

use stdweb::web::{document, Document, IParentNode};
use stdweb::web::HtmlElement;
use stdweb::web::IHtmlElement;

use yew_router::components::RouterAnchor;
use yew::prelude::*;
use crate::app::AppRoute;

use super::component::{Manga, TopBar};
use super::{MangaModel, ChapterModel};
use stdweb::unstable::TryInto;

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
    chapter_no: String,
    chapter: ChapterModel,
    current_page: usize,
    double_page: bool,
}

pub enum Msg {
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
            chapter_no: props.chapter,
            chapter: ChapterModel{
                chapter: "".to_string(),
                url: "".to_string(),
                pages: vec![]
            },
            current_page: 0,
            double_page: false,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.get_chapter();
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ChapterReady(data) => {
                self.chapter = data;
            }
            Msg::PageForward => {
                if self.double_page {
                    self.current_page += 2;
                } else {
                    self.current_page += 1;
                }
                info!("{}", self.current_page);
            }
            Msg::PagePrevious => {
                if self.double_page {
                    self.current_page -= 2;
                } else {
                    self.current_page -= 1;
                }

                info!("{}", self.current_page);
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
            <div class="manga-reader-container">
                <button class="manga-navigate-left" onclick=self.link.callback(|_| Msg::PagePrevious)/>
                <button class="manga-navigate-right" onclick=self.link.callback(|_| Msg::PageForward)/>
                <div class="manga-page-container">
                    {
                        if self.chapter.pages.len() > 0 {
                            if self.double_page {
                                html! {
                                    <>
                                        <img class="manga-page" src=self.chapter.pages[self.current_page]/>
                                        <img class="manga-page" src=self.chapter.pages[self.current_page+1]/>
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
    fn get_chapter(&mut self) {
        let req = Request::get(format!("/api/source/{}/manga/{}/chapter/{}", self.source, self.title, self.chapter_no))
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
}
