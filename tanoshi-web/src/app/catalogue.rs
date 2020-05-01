use super::component::Manga;
use serde::Deserialize;
use web_sys::HtmlElement;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use yew_router::prelude::{Route, RouteAgent};
use yew_router::{router::Router, Switch};

use super::component::model::{FavoriteManga,GetFavoritesResponse};
use tanoshi::manga::{GetMangasResponse, Manga as MangaModel};
use super::component::Spinner;
use super::select::Select;
use super::source::Source;

use http::{Request, Response};
use std::borrow::BorrowMut;
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::utils::{document, window};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Switch, Debug, Clone)]
pub enum CatalogueRoute {
    #[to = "/{source}"]
    Source(String),
    #[to = "/!"]
    Select,
}

#[derive(Clone, Properties)]
pub struct Props {
    pub route: CatalogueRoute,
}

pub struct Catalogue {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    is_fetching: bool,
    token: String,
    route: CatalogueRoute,
}

pub enum Msg {
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
        Catalogue {
            fetch_task: None,
            link,
            route: props.route,
            is_fetching: false,
            token,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Noop => {
                info!("Noop");
            }
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.route = props.route;
        true
    }

    fn view(&self) -> Html {
        let route = self.route.clone();
        html! {
            match route {
                CatalogueRoute::Source(source) => {
                    html! {<Source source=source />}
                },
                CatalogueRoute::Select => {
                    html! {<Select />}
                }
            }
        }
    }
}
