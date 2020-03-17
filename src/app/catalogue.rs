use super::component::Manga;
use serde::Deserialize;
use web_sys::HtmlElement;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use super::component::model::{FavoriteManga, GetFavoritesResponse, GetMangasResponse, MangaModel};
use super::component::Spinner;
use http::{Request, Response};
use std::borrow::BorrowMut;
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::utils::{document, window};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Properties)]
pub struct Props {
    pub source: String,
}

pub struct Catalogue {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    source: String,
    page: i32,
    mangas: Vec<MangaModel>,
    favorites: Vec<String>,
    is_fetching: bool,
    token: String,
    closure: Closure<dyn Fn()>,
}

pub enum Msg {
    MangaReady(GetMangasResponse),
    FavoritesReady(GetFavoritesResponse),
    ScrolledDown,
    Noop,
}

impl Component for Catalogue {
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
        let tmp_link = link.clone();
        let closure = Closure::wrap(Box::new(move || {
            let current_scroll = window().scroll_y().expect("error get scroll y")
                + window().inner_height().unwrap().as_f64().unwrap();
            let height = document()
                .get_element_by_id("catalogue")
                .expect("should have catalogue")
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .offset_height() as f64;
            info!(
                "body scroll: {}, document scroll: {}",
                current_scroll, height
            );
            if current_scroll >= height {
                tmp_link.send_message(Msg::ScrolledDown);
            }
        }) as Box<dyn Fn()>);
        Catalogue {
            fetch_task: None,
            link,
            source: props.source,
            page: 1,
            mangas: vec![],
            favorites: vec![],
            is_fetching: false,
            token,
            closure,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        window().set_onscroll(Some(self.closure.as_ref().unchecked_ref()));
        self.fetch_favorites();
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(data) => {
                let mut mangas = data.mangas;
                self.mangas.append(&mut mangas);
                self.is_fetching = false;
            }
            Msg::FavoritesReady(data) => {
                self.favorites = data
                    .favorites
                    .unwrap()
                    .iter()
                    .map(|ch| ch.title.clone())
                    .collect();
                self.fetch_mangas();
            }
            Msg::ScrolledDown => {
                if !self.is_fetching {
                    self.page += 1;
                    self.fetch_mangas();
                }
            }

            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container mx-auto pb-20"  style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <Spinner is_active=self.is_fetching />
                <div class="grid grid-cols-3 md:grid-cols-6 gap-2" id="catalogue">
                { for self.mangas.iter().map(|manga| html!{
                <Manga
                    title=manga.title.to_owned()
                    thumbnail=manga.thumbnail_url.to_owned()
                    path=manga.path.to_owned()
                    source=self.source.to_owned()
                    is_favorite=self.favorites.contains(&manga.title.to_owned()) />
                })
                }
                </div>
            </div>
        }
    }

    fn destroy(&mut self) {
        window().set_onscroll(None);
    }
}

impl Catalogue {
    fn fetch_mangas(&mut self) {
        let req = Request::get(format!(
            "/api/source/{}?sort_by=popularity&sort_order=descending&page={}",
            self.source, self.page
        ))
        .body(Nothing)
        .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetMangasResponse, anyhow::Error>>>| {
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
            self.is_fetching = true;
        }
    }

    fn fetch_favorites(&mut self) {
        let req = Request::get("/api/favorites")
            .header("Authorization", self.token.clone())
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
        }
    }
}
