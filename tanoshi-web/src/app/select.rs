use web_sys::HtmlElement;
use yew::format::{Json, Nothing, Text};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use std::collections::HashMap;

use super::component::Spinner;
use http::{Request, Response};
use yew::utils::window;

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
    available_sources: Vec<SourceModel>,
    installed_sources: Vec<SourceModel>,
    is_fetching: bool,
    active_tab: Tab,
    button_refs: Vec<NodeRef>,
}

pub enum Msg {
    SourceReady(GetSourceResponse),
    ChangeToAvailableTab,
    ChangeToInstalledTab,
    InstallExtension(usize),
    ExtensionInstalled,
    Noop,
}

impl Component for Select {
    type Message = Msg;
    type Properties = Props;

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Select {
            fetch_task: None,
            link,
            available_sources: vec![],
            installed_sources: vec![],
            is_fetching: false,
            active_tab: Tab::Installed,
            button_refs: vec![NodeRef::default(), NodeRef::default()],
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
                match self.active_tab {
                    Tab::Installed => self.installed_sources = data.sources,
                    Tab::Available => self.available_sources = data.sources,
                };
                self.is_fetching = false;
            }
            Msg::ChangeToInstalledTab => {
                self.active_tab = Tab::Installed;
                self.fetch_sources();
                if let Some(button) = self.button_refs[0].cast::<HtmlElement>() {
                    button
                        .class_list()
                        .add_1("bg-tachiyomi-blue-darker")
                        .expect("failed add class");
                }
                if let Some(button) = self.button_refs[1].cast::<HtmlElement>() {
                    button
                        .class_list()
                        .remove_1("bg-tachiyomi-blue-darker")
                        .expect("failed remove class");
                }
            }
            Msg::ChangeToAvailableTab => {
                self.active_tab = Tab::Available;
                self.fetch_sources();
                if let Some(button) = self.button_refs[0].cast::<HtmlElement>() {
                    button
                        .class_list()
                        .remove_1("bg-tachiyomi-blue-darker")
                        .expect("failed remove class");
                }
                if let Some(button) = self.button_refs[1].cast::<HtmlElement>() {
                    button
                        .class_list()
                        .add_1("bg-tachiyomi-blue-darker")
                        .expect("failed add class");
                }
            }
            Msg::InstallExtension(index) => {
                self.install_source(self.available_sources[index].name.clone());
            }
            Msg::ExtensionInstalled => {
                log::info!("extension installed");
                self.is_fetching = false;
            }
            Msg::Noop => {}
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container mx-auto pb-20 pt-12">
                <div class="w-full px-2 pb-2 flex justify-around block fixed inset-x-0 top-0 z-50 bg-tachiyomi-blue shadow" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                    <button ref=self.button_refs[0].clone()
                        onclick=self.link.callback(|_| Msg::ChangeToInstalledTab)
                        class="hover:bg-tachiyomi-blue-darker rounded flex-grow bg-tachiyomi-blue-darker">
                        <span class="text-white m-1">{"Installed"}</span>
                    </button>
                    <button  ref=self.button_refs[1].clone()
                        onclick=self.link.callback(|_| Msg::ChangeToAvailableTab)
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
                for self.installed_sources.iter().map(|source| html!{
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
        html! {
            <div class="flex flex-col rounded-lg border border-grey-light mx-2 shadow">
            {
                for (0..self.available_sources.len()).map(|i| html!{
                    <div
                        class="flex inline-flex justify-between border-b border-gray-light p-2 content-center hover:bg-gray-200">
                        <span class="text-lg font-semibold">{self.available_sources[i].name.clone()}</span>
                        <button class="bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold px-4 rounded"
                            disabled={self.is_installed(self.available_sources[i].name.clone())}
                            onclick={self.link.callback(move |_| Msg::InstallExtension(i))}>
                            {if !self.is_installed(self.available_sources[i].name.clone()) {"Install"} else {"Installed"}}
                        </button>
                    </div>
                })
            }
            </div>
        }
    }

    fn is_installed(&self, name: String) -> bool {
        self.installed_sources
            .clone()
            .iter()
            .find(|s| s.name == name)
            .is_some()
    }

    fn fetch_sources(&mut self) {
        let url = match self.active_tab {
            Tab::Installed => "/api/source/installed",
            Tab::Available => "/api/source/available",
            _ => "",
        };
        let req = Request::get(url)
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

    fn install_source(&mut self, name: String) {
        let req = Request::post(format!("/api/source/install/{}", name))
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Text>| {
                if let (meta, Ok(_)) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::ExtensionInstalled;
                    }
                }
                Msg::Noop
            }),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
            self.is_fetching = true;
        }
    }
}
