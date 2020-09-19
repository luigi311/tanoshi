use yew::format::{Json, Nothing, Text};
use yew::prelude::*;
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::{FetchService, StorageService};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use super::component::{Filter, Manga, MangaList, Spinner, TopBar, WeakComponentLink};
use tanoshi_lib::manga::{Manga as MangaModel, Params, SortByParam, SortOrderParam};
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
            }
            Msg::Filter => {
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
           <div class="pb-20 sm:pb-25 flex justify-center px-2" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <TopBar>
                    <button onclick=self.link.callback(|_| Msg::Filter) class="hover:bg-accent-darker focus:bg-accent-darker rounded flex-none">
                        <svg fill="none" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" class="mx-2 self-center flex-none"><path d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"></path></svg>
                    </button>
                    <span class="mx-2 my-1 flex-grow text-center text-white">{"Favorites"}</span>
                    <button
                        onclick=self.link.callback(|_| Msg::SyncUpdates)
                        class="hover:bg-accent-darker focus:bg-accent-darker rounded flex-none">
                        <svg viewBox="0 0 20 20" fill="currentColor" class="refresh w-6 h-6 mx-2 self-center flex-none">
                            <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd"></path>
                        </svg>
                    </button>
                </TopBar>
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
            if let Ok(task) = super::api::fetch_manga_chapter(
                manga_id,
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
        let params = Params {
            keyword: None,
            sort_by: Some(self.sort_by.clone()),
            sort_order: Some(self.sort_order.clone()),
            page: None,
            genres: None,
            refresh: None,
        };

        if let Ok(task) = super::api::fetch_favorites(
            params,
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
