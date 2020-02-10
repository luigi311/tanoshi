use serde::{Deserialize, Serialize};
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew::format::{Json, Nothing, Text};
use yew::html::{ChildrenRenderer, NodeRef};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use std::collections::HashMap;

use super::component::{Manga, TopBar};
use super::{MangaModel, Chapter};

#[derive(Clone, Properties)]
pub struct Props {
    pub source: String,
    pub title: String,
}

pub struct Detail {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    source: String,
    title: String,
    manga: MangaModel,
}

pub enum Msg {
    MangaReady(MangaModel),
    FetchReady(Response<Text>),
    Noop,
}

impl Component for Detail {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Detail {
            fetch_task: None,
            link,
            source: props.source,
            title: props.title,
            manga: MangaModel{
                title: "".to_string(),
                author: "".to_string(),
                genre: vec![],
                status: "".to_string(),
                description: "".to_string(),
                url: "".to_string(),
                thumbnail_url: "".to_string(),
                chapter: Default::default()
            },
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.get_manga_info();
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(data) => {
                self.manga = data;
                info!("{:?}", self.manga);
            }
            Msg::FetchReady(data) => {
                info!("fetch ready");
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
            <div class="pure-g">
                <div class="pure-u-2-5"><img src=self.manga.thumbnail_url /></div>
                <div class="pure-u-3-5">
                    <div>{self.manga.title.to_owned()}</div>
                    <div>{self.manga.description.to_owned()}</div>
                </div>
            </div>
            </>
        }
    }
}

impl Detail {
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
}
