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

pub enum Tab {
    Installed,
    Available,
}

#[derive(Clone, Properties)]
pub struct Props {}

pub struct Select {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    sources: Vec<SourceModel>,
    is_fetching: bool,
    active_tab: Tab,
}

pub enum Msg {
    SourceReady(GetSourceResponse),
    ChangeToAvailableTab,
    ChangeToInstalledTab,
    Noop,
}

impl Component for Select {
    type Message = Msg;
    type Properties = Props;

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Select {
            fetch_task: None,
            link,
            sources: vec![],
            is_fetching: false,
            active_tab: Tab::Installed,
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
            Msg::ChangeToInstalledTab => self.active_tab = Tab::Installed,
            Msg::ChangeToAvailableTab => self.active_tab = Tab::Available,
            Msg::Noop => {}
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container mx-auto pb-20 pt-12">
                <div class="w-full px-2 pb-2 flex justify-around block fixed inset-x-0 top-0 z-50 bg-tachiyomi-blue shadow" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                    <button onclick=self.link.callback(|_| Msg::ChangeToInstalledTab)
                        class="hover:bg-tachiyomi-blue-darker rounded flex-grow">
                        <span class="text-white m-1">{"Installed"}</span>
                    </button>
                    <button onclick=self.link.callback(|_| Msg::ChangeToAvailableTab)
                        class="hover:bg-tachiyomi-blue-darker rounded flex-grow">
                        <span class="text-white m-1">{"Available"}</span>
                    </button>
                </div>
                <Spinner is_active=self.is_fetching is_fullscreen=true />
                {
                    match self.active_tab {
                        Tab::Installed => self.installed_view(),
                        Tab::Available => self.available_view(),
                    }
                }
            </div>
        }
    }

    fn destroy(&mut self) {
        window().set_onscroll(None);
    }
}

impl Select {
    fn installed_view(&self) -> Html {
        html! {
            <div class="flex flex-col rounded-lg border border-grey-light mx-2 shadow">
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
        }
    }

    fn available_view(&self) -> Html {
        html! {}
    }

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
