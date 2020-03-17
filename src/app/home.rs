use serde::Deserialize;
use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::{FetchService, StorageService};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use super::component::model::{FavoriteManga, GetFavoritesResponse, GetMangasResponse};
use super::component::{Manga, Spinner};

#[derive(Deserialize, Debug)]
pub struct MangaModel {
    pub title: String,
    pub thumbnail_url: String,
}

#[derive(Clone, Properties)]
pub struct Props {}

#[derive(Deserialize)]
pub struct Token {
    token: String,
}

pub struct Home {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    mangas: Vec<FavoriteManga>,
    token: String,
    is_fetching: bool,
}

pub enum Msg {
    FavoritesReady(GetFavoritesResponse),
    Noop,
}

impl Component for Home {
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

        Home {
            fetch_task: None,
            link,
            mangas: vec![],
            token,
            is_fetching: false,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.fetch_favorites();
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FavoritesReady(data) => {
                self.mangas = data.favorites.unwrap();
                self.is_fetching = false;
            }
            Noop => {
                return false;
            }
        };
        true
    }

    fn view(&self) -> Html {
        html! {
           <div class="container mx-auto pb-20" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <Spinner is_active=self.is_fetching />
                <div class="grid grid-cols-3 md:grid-cols-5 lg:grid-cols-7 gap-2">
                {
                for self.mangas.iter().map(|manga|  html!{
                <Manga
                    title=manga.title.to_owned()
                    thumbnail=manga.thumbnail_url.to_owned()
                    path=manga.path.to_owned()
                    source=manga.source.to_owned()
                    is_favorite={false} />
                })
                }
                </div>
            </div>
        }
    }
}

impl Home {
    fn remainder(&self, len: usize) -> Vec<i32> {
        let end = (3 - (len % 3)) as i32;
        return (0..end).collect();
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
