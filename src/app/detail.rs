use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use super::component::model::{ChapterModel, GetChaptersResponse, GetMangaResponse, MangaModel};
use super::component::Spinner;
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
    is_fetching_manga: bool,
    is_fetching_chapter: bool,
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
            is_fetching_manga: false,
            is_fetching_chapter: false,
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
                self.get_chapters();
                self.is_fetching_manga = false;
            }
            Msg::ChapterReady(data) => {
                self.chapters = data.chapters;
                self.is_fetching_chapter = false;
            }
            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container pb-10">
            <Spinner is_active={self.is_fetching_manga || self.is_fetching_chapter} />
            <div class="w-2/3 mx-auto flex sm:flex-wrap">
                <div class="flex-none m-w">
                    <div class="manga-cover-container">
                        <img class="manga-cover" src=self.manga.thumbnail_url />
                    </div>
                </div>
                <div class="flex flex-col m-2 p-4">
                    <span class="text-2xl font-bold">{self.manga.title.to_owned()}</span>
                    <span class="font-semibold">{self.manga.status.to_owned()}</span>
                    <span class="font-medium">{self.manga.genre.join(", ").to_owned()}</span>
                    <p>{self.manga.description.to_owned()}</p>
                </div>
            </div>
            <div class="w-2/3 mx-auto">
                <div class="bg-white shadow-md rounded my-6">
                    <table class="text-left w-full border-collapse">
                    <thead>
                        <tr>
                         <th class="py-4 px-6 bg-grey-lightest font-bold uppercase text-sm text-grey-dark border-b border-grey-light">
                            {"Read Chapter"}
                         </th>
                        </tr>
                    </thead>
                    <tbody>
                        {
                            for self.chapters.iter().map(|(chapter)| html!{
                                <tr class="hover:bg-grey-lighter">
                                    <td class="py-4 px-6 border-b border-grey-light">
                                        <RouterAnchor<AppRoute>  route=AppRoute::Chapter(self.source.to_owned(), self.title.to_owned(), chapter.no.to_owned(), 1)>
                                            {format!("Chapter {}", chapter.no.to_owned())}
                                        </RouterAnchor<AppRoute>>
                                    </td>
                                </tr>
                            })
                        }
                    </tbody>
                    </table>
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
            self.is_fetching_manga = true;
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
            self.is_fetching_chapter = true;
        }
    }
}
