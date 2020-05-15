use yew::services::storage::Area;
use yew::services::StorageService;
use yew::{
    html, Bridge, Bridged, Component, ComponentLink, Html, Properties, ShouldRender,
};
use yew_router::prelude::{Route, RouteAgent};
use yew_router::{router::Router, Switch};

use crate::app::catalogue::{CatalogueRoute};

use super::catalogue::Catalogue;
use super::component::NavigationBar;
use super::detail::Detail;
use super::history::History;
use super::home::Home;
use super::settings::Settings;

#[derive(Switch, Debug, Clone)]
pub enum BrowseRoute {
    #[to = "/catalogue/{source}/manga/{title}"]
    Detail(String, String),
    #[to = "/catalogue{*:rest}"]
    Catalogue(CatalogueRoute),
    #[to = "/updates"]
    Updates,
    #[to = "/history"]
    History,
    #[to = "/settings"]
    Settings,
    #[to = "/"]
    Home,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub source: Option<String>,
    pub title: Option<String>,
}

impl Into<Props> for BrowseRoute {
    fn into(self) -> Props {
        match self {
            BrowseRoute::Detail(source, title) => Props {
                source: Some(source),
                title: Some(title),
            },
            _ => Props {
                source: None,
                title: None,
            },
        }
    }
}

pub struct Browse {
    props: Props,
    link: ComponentLink<Self>,
    storage: StorageService,
    router: Box<dyn Bridge<RouteAgent>>,
    route: String,
}

pub enum Msg {
    RouterCallback(Route),
}

impl Component for Browse {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let callback = link.callback(|route| Msg::RouterCallback(route));
        let router = RouteAgent::bridge(callback);
        Browse {
            props,
            link,
            storage,
            router,
            route: "/".to_string(),
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            return true;
        }
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::RouterCallback(route) => {
                self.route = route.route;
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
                <>
                    <div class="block fixed inset-x-0 top-0 z-50 bg-tachiyomi-blue safe-top z-50 shadow"></div>
                    <NavigationBar />
                    <Router<BrowseRoute>
                    render = Router::render(|switch: BrowseRoute| {
                    match switch {
                        BrowseRoute::Detail(source, title) => html!{<Detail source=source title=title/>},
                        BrowseRoute::Catalogue(catalogue_route) => html!{<Catalogue route=catalogue_route/>},
                        BrowseRoute::Updates => html!{<History/>},
                        BrowseRoute::History => html!{<History/>},
                        BrowseRoute::Home => html!{<Home/>},
                        BrowseRoute::Settings => html!{<Settings />},
                    }}) / >
            < / >
        }
    }
}