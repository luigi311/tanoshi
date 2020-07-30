use yew::format::{Json, Nothing, Text};
use yew::prelude::*;
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::{FetchService, StorageService};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use super::component::{Manga, MangaList, Spinner, WeakComponentLink, Filter};
use tanoshi_lib::manga::{Manga as MangaModel, SortByParam, SortOrderParam, Params};
use tanoshi_lib::rest::GetMangasResponse;

#[derive(Clone, Properties)]
pub struct Props {}

pub struct Home {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    mangas: Vec<MangaModel>,
    token: String,
    is_fetching: bool,
    should_fetch: bool,
    update_queue: Vec<i32>,
    show_filter: bool,
    sort_by: SortByParam,
    sort_order: SortOrderParam,
}

pub enum Msg {
    FavoritesReady(GetMangasResponse),
    SyncUpdates,
    MangaUpdated,
    Filter,
    FilterClosed,
    FilterCancel,
    SortByChange(SortByParam),
    SortOrderChange(SortOrderParam),
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
            show_filter: false,
            sort_by: SortByParam::Title,
            sort_order: SortOrderParam::Asc,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FavoritesReady(data) => {
                self.mangas = data.mangas;

                self.is_fetching = false;
                self.fetch_task = None;
            }
            Msg::SyncUpdates => {
                if self.update_queue.is_empty() {
                    self.update_queue = self.mangas.iter().map(|m| m.id).collect();
                    self.fetch_manga_chapter();
                }
            }
            Msg::MangaUpdated => {
                self.fetch_manga_chapter();
            }Msg::Filter => {
                if !self.show_filter {
                    self.show_filter = true;
                } else {
                    return false;
                }
            }
            Msg::FilterClosed => {
                if self.show_filter {
                    self.mangas.clear();
                    self.fetch_favorites();
                    self.show_filter = false;
                } else {
                    return false;
                }
            }
            Msg::FilterCancel => {
                self.show_filter = false;
            }
            Msg::SortByChange(sort_by) => {
                self.sort_by = sort_by;
            }
            Msg::SortOrderChange(sort_order) => {
                self.sort_order = sort_order;
            }
            Msg::Noop => {
                return false;
            }
        };
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let list_link = &WeakComponentLink::<MangaList>::default();
        html! {
           <div class="container mx-auto pb-20 sm:pb-25" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <div class="w-full px-2 pb-2 flex justify-between block fixed inset-x-0 top-0 z-50 bg-tachiyomi-blue shadow" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                    <button onclick=self.link.callback(|_| Msg::Filter) class="hover:bg-tachiyomi-blue-darker focus:bg-tachiyomi-blue-darker rounded flex-none">
                        <svg fill="none" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" class="mx-2 self-center flex-none"><path d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"></path></svg>
                    </button>
                    <span class="mx-2 my-1 flex-grow text-center text-white">{"Favorites"}</span>
                    <button
                        onclick=self.link.callback(|_| Msg::SyncUpdates)
                        class="hover:bg-tachiyomi-blue-darker focus:bg-tachiyomi-blue-darker rounded flex-none">
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" class="mx-2 my-auto self-center">
                            <path class="heroicon-ui" d="M6 18.7V21a1 1 0 0 1-2 0v-5a1 1 0 0 1 1-1h5a1 1 0 1 1 0 2H7.1A7 7 0 0 0 19 12a1 1 0 1 1 2 0 9 9 0 0 1-15 6.7zM18 5.3V3a1 1 0 0 1 2 0v5a1 1 0 0 1-1 1h-5a1 1 0 0 1 0-2h2.9A7 7 0 0 0 5 12a1 1 0 1 1-2 0 9 9 0 0 1 15-6.7z"/>
                        </svg>
                    </button>
                </div>
                <Spinner is_active=self.is_fetching is_fullscreen=true />
                <MangaList weak_link=list_link>
                    { for self.mangas.iter().map(|manga| {
                        html_nested!{
                        <Manga
                            key=manga.id
                            id=manga.id
                            title=&manga.title
                            thumbnail=&manga.thumbnail_url
                            is_favorite=false />
                    }})
                    }
                </MangaList>
                <Filter
                    show={self.show_filter}
                    onsortbychange={self.link.callback(|data| Msg::SortByChange(data))}
                    onsortorderchange={self.link.callback(|data| Msg::SortOrderChange(data))}
                    onclose={self.link.callback(|_| Msg::FilterClosed)}
                    oncancel={self.link.callback(|_| Msg::FilterCancel)}
                    sort_by={&self.sort_by}
                    sort_order={&self.sort_order}
                />
            </div>
        }
    }

    fn rendered(&mut self, _first_render: bool) {
        if self.should_fetch {
            self.fetch_favorites();
            self.should_fetch = false;
        }
    }
}

impl Home {
    fn fetch_manga_chapter(&mut self) {
        if let Some(manga_id) = self.update_queue.pop() {
            let req = Request::get(format!("/api/manga/{}/chapter?refresh=true", manga_id))
                .header("Authorization", self.token.to_string())
                .body(Nothing)
                .expect("failed to build request");

            if let Ok(task) = FetchService::fetch(
                req,
                self.link
                    .callback(|_response: Response<Text>| Msg::MangaUpdated),
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
        let params = serde_urlencoded::to_string(Params {
            keyword: None,
            sort_by: Some(self.sort_by.clone()),
            sort_order: Some(self.sort_order.clone()),
            page: None,
            genres: None,
            refresh: None,
        }).unwrap();
        let req = Request::get(format!("/api/favorites?{}", params))
            .header("Authorization", self.token.to_string())
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetMangasResponse, anyhow::Error>>>| {
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
