use serde::Deserialize;
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{Request, Response, FetchTask};
use yew::services::{FetchService, StorageService};
use super::component::{Manga};

use crate::app::{GetMangasResponse, GetFavoritesResponse, FavoriteManga};
use yew::services::storage::Area;

#[derive(Deserialize, Debug)]
pub struct MangaModel {
    pub title: String,
    pub thumbnail_url: String,
}

#[derive(Clone, Properties)]
pub struct Props {

}

#[derive(Deserialize)]
pub struct Token {
    token: String,
}

pub struct Home {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    mangas: Vec<FavoriteManga>,
    token: Token,
}

pub enum Msg {
    FavoritesReady(GetFavoritesResponse),
    Noop,
}

impl Component for Home {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local);
        let token = {
          if let Json(Ok(token)) = storage.restore("token") {
              token
          }   else {
                Token{token: "".to_string()}
            }
        };

        Home {
            fetch_task: None,
            link,
            mangas: vec![],
            token,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FavoritesReady(data) => {
                self.mangas = data.favorites.unwrap();
            }
            Noop => {
                return false;
            },
        };
        true
    }

    fn mounted(&mut self) -> ShouldRender {
        self.fetch_favorites();
        true
    }

    fn view(&self) -> Html {
        html! {
           <div class="container-fluid">
                <div class="row row-cols-sm-1 row-cols-md-3 row-cols-lg-4 row-cols-xl-5">
                { for self.mangas.iter().map(|manga|  html!{
                <Manga
                    title=manga.title.to_owned()
                    thumbnail=manga.thumbnail_url.to_owned()
                    path=manga.path.to_owned()
                    source=manga.source.to_owned() />
                }) }
                </div>
            </div>
        }
    }
}

impl Home {
    fn fetch_favorites(&mut self) {
        let req = Request::get("/api/favorites")
            .header("Authorization", self.token.token.to_string())
            .body(Nothing).expect("failed to build request");

        let task = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Json<Result<GetFavoritesResponse, anyhow::Error>>>| {
                if let (meta, Json(Ok(data))) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::FavoritesReady(data);
                    }
                }
                Msg::Noop
            }));
        self.fetch_task = Some(task);
    }
}
