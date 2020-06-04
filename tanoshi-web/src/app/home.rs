use serde::Deserialize;
use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::{FetchService, StorageService};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use super::component::{Manga, Spinner};
use tanoshi_lib::manga::{FavoriteManga, GetFavoritesResponse};

#[derive(Deserialize, Debug)]
pub struct MangaModel {
    pub title: String,
    pub thumbnail_url: String,
}

#[derive(Clone, Properties)]
pub struct Props {}

pub struct Home {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    mangas: Vec<FavoriteManga>,
    token: String,
    is_fetching: bool,
    should_fetch: bool,
    update_queue: Vec<i32>,
}

pub enum Msg {
    FavoritesReady(GetFavoritesResponse),
    SyncUpdates,
    MangaUpdated,
    Noop,
}

impl Component for Home {
    type Message = Msg;
    type Properties = Props;

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let token = {
            if let Ok(token) = storage.restore("token") {
                token
            } else {
                "".to_string()
            }
        };

        Home {
            fetch_task: None,
            link,
            mangas: vec![],
            token,
            is_fetching: false,
            should_fetch: true,
            update_queue: vec![],
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn rendered(&mut self, _first_render: bool) {
        if self.should_fetch {
            self.fetch_favorites();
            self.should_fetch = false;
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FavoritesReady(data) => {
                self.mangas = data.favorites.unwrap();
                self.is_fetching = false;
                self.fetch_task = None;
            }
            Msg::SyncUpdates => {
                if self.update_queue.is_empty() {
                    self.update_queue = self.mangas.iter().map(|m| m.manga_id).collect();
                    self.fetch_manga_chapter();
                }
            }
            Msg::MangaUpdated => {
                self.fetch_manga_chapter();
            }
            Msg::Noop => {
                return false;
            }
        };
        true
    }

    fn view(&self) -> Html {
        html! {
           <div class="container mx-auto pb-20 sm:pb-25" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <div class="w-full px-2 pb-2 flex justify-between block fixed inset-x-0 md:top-0 z-50 bg-tachiyomi-blue shadow" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" class="mx-2 self-center flex-none">
                        <path class="heroicon-ui" d="M4.06 13a8 8 0 0 0 5.18 6.51A18.5 18.5 0 0 1 8.02 13H4.06zm0-2h3.96a18.5 18.5 0 0 1 1.22-6.51A8 8 0 0 0 4.06 11zm15.88 0a8 8 0 0 0-5.18-6.51A18.5 18.5 0 0 1 15.98 11h3.96zm0 2h-3.96a18.5 18.5 0 0 1-1.22 6.51A8 8 0 0 0 19.94 13zm-9.92 0c.16 3.95 1.23 7 1.98 7s1.82-3.05 1.98-7h-3.96zm0-2h3.96c-.16-3.95-1.23-7-1.98-7s-1.82 3.05-1.98 7zM12 22a10 10 0 1 1 0-20 10 10 0 0 1 0 20z"/>
                    </svg>
                    <span class="mx-2 mb-2 flex-grow text-center text-white">{"Favorites"}</span>
                    <button
                        onclick=self.link.callback(|_| Msg::SyncUpdates)
                        class="hover:bg-tachiyomi-blue-darker focus:bg-tachiyomi-blue-darker rounded flex-none">
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" class="mx-2 my-auto self-center">
                            <path class="heroicon-ui" d="M6 18.7V21a1 1 0 0 1-2 0v-5a1 1 0 0 1 1-1h5a1 1 0 1 1 0 2H7.1A7 7 0 0 0 19 12a1 1 0 1 1 2 0 9 9 0 0 1-15 6.7zM18 5.3V3a1 1 0 0 1 2 0v5a1 1 0 0 1-1 1h-5a1 1 0 0 1 0-2h2.9A7 7 0 0 0 5 12a1 1 0 1 1-2 0 9 9 0 0 1 15-6.7z"/>
                        </svg>
                    </button>
                </div>
                <Spinner is_active=self.is_fetching is_fullscreen=true />
                <div class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2 mt-12">
                {
                for self.mangas.iter().map(|manga| html!{
                <Manga
                    id=manga.manga_id
                    title=manga.title.to_owned()
                    thumbnail=manga.thumbnail_url.to_owned()
                    is_favorite={false} />
                })
                }
                </div>
            </div>
        }
    }
}

impl Home {
    fn fetch_manga_chapter(&mut self) {
        if let Some(manga_id) = self.update_queue.pop() {
            let req = Request::get(format!("/api/manga/{}/chapter", manga_id))
                .header("Authorization", self.token.to_string())
                .body(Nothing)
                .expect("failed to build request");

            if let Ok(task) = FetchService::new().fetch(
                req,
                self.link.callback(|response: Response<Text>| {
                    if let (meta, Ok(_)) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::MangaUpdated;
                        }
                    }
                    Msg::Noop
                }),
            ) {
                self.fetch_task = Some(FetchTask::from(task));
                self.is_fetching = true;
            }
        } else {
            self.fetch_task = None;
            self.is_fetching = false;
        }
    }

    fn fetch_favorites(&mut self) {
        let req = Request::get("/api/favorites")
            .header("Authorization", self.token.to_string())
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetFavoritesResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::FavoritesReady(data);
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
}
