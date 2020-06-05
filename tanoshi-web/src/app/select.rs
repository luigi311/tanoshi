use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use super::component::Spinner;
use http::{Request, Response};
use yew::utils::window;

use super::app::AppRoute;
use super::browse::BrowseRoute;
use super::catalogue::CatalogueRoute;

use tanoshi_lib::manga::{GetSourceResponse, Source as SourceModel};

#[derive(Clone, Properties)]
pub struct Props {}

pub struct Select {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    sources: Vec<SourceModel>,
    is_fetching: bool,
}

pub enum Msg {
    SourceReady(GetSourceResponse),
    Noop,
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

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.fetch_sources();
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SourceReady(data) => {
                self.sources = data.sources;
                self.is_fetching = false;
            }
            Msg::Noop => {}
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container mx-auto pb-20"  style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <Spinner is_active=self.is_fetching is_fullscreen=true />
                <div class="flex flex-col rounded-lg border border-grey-light m-2 shadow">
                {
                    for self.sources.iter().map(|source| html!{
                        <RouterAnchor<BrowseRoute>
                            classes="flex inline-flex border-b border-gray-light p-2 content-center hover:bg-gray-200"
                            route=BrowseRoute::Catalogue(CatalogueRoute::Source(source.id))>
                            <span class="text-lg font-semibold">{source.name.to_owned()}</span>
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
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
            self.is_fetching = true;
        }
    }
}
