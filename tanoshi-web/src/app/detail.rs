use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use super::component::Spinner;
use crate::app::{job, AppRoute};

use tanoshi_lib::manga::{
    Chapter as ChapterModel, GetChaptersResponse, GetMangaResponse, Manga as MangaModel,
};

use serde::{Deserialize, Serialize};

use anyhow;
use yew::services::storage::Area;
use yew::services::StorageService;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FavoriteManga {
    pub source: String,
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AddFavoritesResponse {
    pub status: String,
}

#[derive(Clone, Properties)]
pub struct Props {
    pub manga_id: i32,
}

pub struct Detail {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    token: String,
    manga_id: i32,
    manga: MangaModel,
    chapters: Vec<ChapterModel>,
    is_fetching: bool,
    should_fetch: bool,
    worker: Box<dyn Bridge<job::Worker>>,
}

pub enum Msg {
    MangaReady(GetMangaResponse),
    ChapterReady(GetChaptersResponse),
    Refresh,
    FavoriteEvent,
    Favorited(AddFavoritesResponse),
    Unfavorited(AddFavoritesResponse),
    Noop,
}

impl Component for Detail {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let token = {
            if let Ok(token) = storage.restore("token") {
                token
            } else {
                "".to_string()
            }
        };

        let worker_callback = link.callback(|msg| match msg {
            job::Response::MangaFetched(data) => Msg::MangaReady(data),
            job::Response::ChaptersFetched(data) => Msg::ChapterReady(data),
            _ => Msg::Noop,
        });
        let worker = job::Worker::bridge(worker_callback);

        info!("create");
        Detail {
            fetch_task: None,
            link,
            token,
            manga_id: props.manga_id,
            manga: MangaModel::default(),
            chapters: vec![],
            is_fetching: true,
            should_fetch: true,
            worker,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        info!("changed");
        if self.manga_id != props.manga_id {
            self.manga_id = props.manga_id;
            self.should_fetch = true;
            return true;
        }
        false
    }

    fn rendered(&mut self, first_render: bool) {
        info!("rendered {}", first_render);
        if self.should_fetch {
            self.get_manga_info();
            self.should_fetch = false;
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        info!("update");
        match msg {
            Msg::MangaReady(data) => {
                self.manga = data.manga;
                self.get_chapters(false);
                self.is_fetching = false;
            }
            Msg::ChapterReady(data) => {
                self.chapters = data.chapters;
                self.is_fetching = false;
            }
            Msg::Refresh => {
                self.get_chapters(true);
            }
            Msg::FavoriteEvent => {
                if self.manga.is_favorite {
                    self.unfavorite();
                } else {
                    self.favorite();
                }
            }
            Msg::Favorited(data) => {
                if data.status == "success" {
                    self.manga.is_favorite = true;
                }
            }
            Msg::Unfavorited(data) => {
                if data.status == "success" {
                    self.manga.is_favorite = false;
                }
            }
            Msg::Noop => {
                return false;
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container pb-20" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
            <Spinner is_active={self.is_fetching} is_fullscreen=true />
            <div class="m-2 flex md:flex-row sm:flex-col">
                <div class="flex-shrink-0 lg:m-2 sm:mx-auto sm:my-2">
                    <div class="relative my-4">
                        <img class="manga-cover sm:cover cover-sm" src=self.manga.thumbnail_url />
                    </div>
                </div>
                <div class="flex flex-col m-2">
                    <p class="md:text-xl sm:text-base font-bold">{self.manga.title.to_owned()}</p>
                    <p class="md:text-xl sm:text-sm font-semibold">{self.manga.author.to_owned()}</p>
                    <p class="md:text-xl sm:text-sm font-semibold">{self.manga.status.to_owned()}</p>
                    //<p class="md:text-xl sm:text-sm font-medium break-normal">{self.manga.genre.join(", ").to_owned()}</p>
                    <p class="break-normal md:text-base sm:text-xs">{self.manga.description.to_owned()}</p>
                </div>
            </div>
            <div class="inline-flex my-2 ml-4">
                <button
                onclick=self.link.callback(|_| Msg::FavoriteEvent)
                class="inline-flex items-center bg-white hover:bg-gray-100 text-gray-800 font-semibold py-1 px-2 border border-gray-400 rounded shadow">
                    <svg class="w-4 h-4 mr-2" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path class="heroicon-ui" d="M6.1 21.98a1 1 0 0 1-1.45-1.06l1.03-6.03-4.38-4.26a1 1 0 0 1 .56-1.71l6.05-.88 2.7-5.48a1 1 0 0 1 1.8 0l2.7 5.48 6.06.88a1 1 0 0 1 .55 1.7l-4.38 4.27 1.04 6.03a1 1 0 0 1-1.46 1.06l-5.4-2.85-5.42 2.85zm4.95-4.87a1 1 0 0 1 .93 0l4.08 2.15-.78-4.55a1 1 0 0 1 .29-.88l3.3-3.22-4.56-.67a1 1 0 0 1-.76-.54l-2.04-4.14L9.47 9.4a1 1 0 0 1-.75.54l-4.57.67 3.3 3.22a1 1 0 0 1 .3.88l-.79 4.55 4.09-2.15z"/></svg>
                    <span>{if !self.manga.is_favorite {"Favorite"} else {"Unfavorite"}}</span>
                </button>
                <button
                onclick=self.link.callback(|_| Msg::Refresh)
                class="ml-2 inline-flex items-center bg-white hover:bg-gray-100 text-gray-800 font-semibold py-1 px-2 border border-gray-400 rounded shadow">
                    <svg class="fill-current w-4 h-4 mr-2" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path class="heroicon-ui" d="M6 18.7V21a1 1 0 0 1-2 0v-5a1 1 0 0 1 1-1h5a1 1 0 1 1 0 2H7.1A7 7 0 0 0 19 12a1 1 0 1 1 2 0 9 9 0 0 1-15 6.7zM18 5.3V3a1 1 0 0 1 2 0v5a1 1 0 0 1-1 1h-5a1 1 0 0 1 0-2h2.9A7 7 0 0 0 5 12a1 1 0 1 1-2 0 9 9 0 0 1 15-6.7z"/></svg>
                    <span>{"Refresh"}</span>
                </button>
                <RouterAnchor<AppRoute>
                classes="ml-2 inline-flex items-center bg-white hover:bg-gray-100 text-gray-800 font-semibold py-1 px-2 border border-gray-400 rounded shadow"
                route=AppRoute::Chapter(self.manga.last_read.unwrap_or(self.chapters.last().unwrap_or(&ChapterModel::default()).id), (self.manga.last_page.as_ref().unwrap_or(&0) + 1) as usize)>
                    <svg class="fill-current w-4 h-4 mr-2" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path class="heroicon-ui" d="M7 5H5v14h14V5h-2v10a1 1 0 0 1-1.45.9L12 14.11l-3.55 1.77A1 1 0 0 1 7 15V5zM5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5c0-1.1.9-2 2-2zm4 2v8.38l2.55-1.27a1 1 0 0 1 .9 0L15 13.38V5H9z"/></svg>
                    <span>{"Read"}</span>
                </RouterAnchor<AppRoute>>
            </div>
            <div class="w-6/7 mx-2 grid grid-cols-1 lg:grid-cols-2">
                {
                    for self.chapters.iter().map(|(chapter)| html!{
                        <div class={
                            format!("rounded-lg border border-grey-light m-2 {}", if chapter.read > 0 {"bg-gray-400"} else {""})
                        }>
                            <RouterAnchor<AppRoute>
                            classes="px-2 py-2 text-left block hover:shadow"
                            route=AppRoute::Chapter(chapter.id, (chapter.read + 1) as usize)>
                                {format!("Ch. {} {}", chapter.no.to_owned(), chapter.title.to_owned())}
                            </RouterAnchor<AppRoute>>
                        </div>
                    })
                }
            </div>
            </div>
        }
    }
}

impl Detail {
    fn get_manga_info(&mut self) {
        self.worker.send(job::Request::FetchManga(self.manga_id));
        self.is_fetching = true;
    }

    fn get_chapters(&mut self, refresh: bool) {
        self.worker
            .send(job::Request::FetchChapters(self.manga_id, refresh));
        self.is_fetching = true;
    }

    fn favorite(&mut self) {
        let req = Request::post(format!("/api/favorites/manga/{}", self.manga_id))
            .header("Authorization", self.token.to_owned())
            .header("Content-Type", "application/json")
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<AddFavoritesResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::Favorited(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }

    fn unfavorite(&mut self) {
        let req = Request::delete(format!("/api/favorites/manga/{}", self.manga_id))
            .header("Authorization", self.token.to_owned())
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<AddFavoritesResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::Unfavorited(data);
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
