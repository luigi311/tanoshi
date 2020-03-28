use super::component::Manga;
use serde::Deserialize;
use web_sys::HtmlElement;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use super::component::model::{FavoriteManga, GetSourceResponse};
use super::component::Spinner;
use http::{Request, Response};
use std::borrow::BorrowMut;
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::utils::{document, window};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use super::app::AppRoute;
use super::browse::BrowseRoute;
use super::catalogue::CatalogueRoute;

#[derive(Clone, Properties)]
pub struct Props {}

pub struct Select {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    sources: Vec<String>,
    is_fetching: bool,
}

pub enum Msg {
    SourceReady(GetSourceResponse),
    noop,
}

impl Component for Select {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Select {
            fetch_task: None,
            link,
            sources: vec![],
            is_fetching: false,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.fetch_sources();
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SourceReady(data) => {
                self.sources = data.sources;
                self.is_fetching = false;
            }
            Msg::noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container mx-auto pb-20"  style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <Spinner is_active=self.is_fetching />
                <div class="flex flex-col rounded-lg border border-grey-light m-2 shadow">
                {
                    for self.sources.iter().map(|source| html!{
                        <RouterAnchor<BrowseRoute>
                            classes="flex inline-flex border-b border-gray-light p-2 content-center hover:bg-gray-200"
                            route=BrowseRoute::Catalogue(CatalogueRoute::Source("mangasee".to_string()))>
                            <span class="text-lg font-semibold">{source.clone()}</span>
                        </RouterAnchor<BrowseRoute>>
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

impl Select {
    fn fetch_sources(&mut self) {
        let req = Request::get("/api/source")
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetSourceResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::SourceReady(data);
                        }
                    }
                    Msg::noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
            self.is_fetching = true;
        }
    }
}
