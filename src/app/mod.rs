use serde::{Deserialize, Serialize};
use yew::{Component, ComponentLink, html, Html, ShouldRender};
use yew::format::{Json, Nothing, Text};
use yew::html::{ChildrenRenderer, NodeRef, Properties};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

use self::component::Manga;

mod component;

#[derive(Deserialize, Debug)]
pub struct MangaModel {
    pub title: String,
    pub thumbnail_url: String,
}

pub struct App {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    mangas: Vec<MangaModel>,
}

pub enum Msg {
    MangaReady(Vec<MangaModel>),
    FetchReady(Response<Text>),
    Noop,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        App {
            fetch_task: None,
            link,
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

impl App {
    fn fetch_mangas(&mut self) {
        info!("fetching");
        let req = Request::get("http://localhost:3030/source/mangasee?sort_by=popularity&sort_order=descending&page=1")
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
