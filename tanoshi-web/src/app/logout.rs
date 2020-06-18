use yew::services::{storage::Area, StorageService};
use yew::{html, Bridge, Bridged, Component, ComponentLink, Html, Properties, ShouldRender};

use yew_router::components::RouterAnchor;
use yew_router::{agent::RouteRequest, prelude::*};

#[derive(Clone, PartialEq, Properties)]
pub struct Props {}

pub struct Logout {
    router: Box<dyn Bridge<RouteAgent>>,
    storage: StorageService,
}

pub enum Msg {
    Noop,
}

impl Component for Logout {
    type Message = Msg;
    type Properties = Props;

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let callback = link.callback(|_| Msg::Noop);
        let router = RouteAgent::bridge(callback);
        Logout { router, storage }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.storage.remove("token");
            self.router
                .send(RouteRequest::ChangeRoute(Route::from("/login".to_string())));
        }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {<></>}
    }
}
