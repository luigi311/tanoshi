use serde::{Deserialize, Serialize};
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew::format::{Json, Nothing, Text};
use yew::html::{ChildrenRenderer, NodeRef};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

use super::component::Manga;

#[derive(Deserialize, Debug)]
pub struct MangaModel {
    pub title: String,
    pub thumbnail_url: String,
}

#[derive(Clone, Properties)]
pub struct Props {
    pub source: String,
}

pub struct Catalogue {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    source: String,
    mangas: Vec<MangaModel>,
}

pub enum Msg {
    MangaReady(Vec<MangaModel>),
    FetchReady(Response<Text>),
    Noop,
}

impl Component for Catalogue {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Catalogue {
            fetch_task: None,
            link,
            source: props.source,
            mangas: vec![],
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(data) => {
                self.mangas = data;
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

    fn mounted(&mut self) -> ShouldRender {
        self.fetch_mangas();
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="pure-g">
            { for self.mangas.iter().map(|manga|  html!{
                <Manga title=manga.title.to_owned() thumbnail=manga.thumbnail_url.to_owned() />
            }) }
            </div>
        }
    }
}

impl Catalogue {
    fn fetch_mangas(&mut self) {
        let req = Request::get(format!("/api/source/{}?sort_by=popularity&sort_order=descending&page=1", self.source))
            .body(Nothing)
            .expect("failed to build request");

        let task = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Json<Result<Vec<MangaModel>, failure::Error>>>| {
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
