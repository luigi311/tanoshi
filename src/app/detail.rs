use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use super::component::model::{ChapterModel, GetChaptersResponse, GetMangaResponse, MangaModel};
use crate::app::AppRoute;

use anyhow;

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
    chapters: Vec<ChapterModel>,
}

pub enum Msg {
    MangaReady(GetMangaResponse),
    ChapterReady(GetChaptersResponse),
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
            manga: MangaModel {
                title: "".to_string(),
                author: "".to_string(),
                genre: vec![],
                status: "".to_string(),
                description: "".to_string(),
                path: "".to_string(),
                thumbnail_url: "".to_string(),
            },
            chapters: vec![],
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.get_manga_info();
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(data) => {
                self.manga = data.manga;
                self.get_chapters()
            }
            Msg::ChapterReady(data) => {
                self.chapters = data.chapters;
            }
            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container">
            <div class="row justify-content-center">
                <div class="col-sm-auto">
                    <div class="manga-cover-container">
                        <img class="manga-cover" src=self.manga.thumbnail_url />
                    </div>
                </div>
                <div class="col-md">
                    <h1>{self.manga.title.to_owned()}</h1>
                    <h4>{self.manga.status.to_owned()}</h4>
                    <h5>{self.manga.genre.join(", ").to_owned()}</h5>
                    <p>{self.manga.description.to_owned()}</p>
                </div>
            </div>
            <div class="row justify-content-center">
                <div class="col-lg">
                    <div class="card" style="width: 100%;">
                        <div class="card-header">
                            {"Read Chapter"}
                        </div>
                        <ul class="list-group">
                        {
                            for self.chapters.iter().map(|(chapter)| html!{
                                <RouterAnchor<AppRoute> route=AppRoute::Chapter(self.source.to_owned(), self.title.to_owned(), chapter.no.to_owned(), 1)>
                                    <li class="list-group-item list-group-item-action">{format!("Chapter {}", chapter.no.to_owned())}</li>
                                </RouterAnchor<AppRoute>>
                            })
                        }
                        </ul>
                    </div>
                </div>
            </div>
            </div>
        }
    }
}

impl Detail {
    fn get_manga_info(&mut self) {
        let req = Request::get(format!("/api/source/{}/manga/{}", self.source, self.title))
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetMangaResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::MangaReady(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }

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
}
