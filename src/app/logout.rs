use serde::{Deserialize, Serialize};
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender, SubmitEvent, InputData, Bridge, Bridged};
use yew::format::Json;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::{StorageService, storage::Area};

use yew_router::{prelude::*,agent::RouteRequest};
use yew_router::components::RouterAnchor;

use stdweb::web::document;
use stdweb::web::event::{IEvent, ClickEvent};

use crate::app::AppRoute;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
}

pub struct Logout {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    router: Box<dyn Bridge<RouteAgent>>,
    storage: StorageService,
}

pub enum Msg {
    Noop,
}

impl Component for Logout {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local);
        let callback = link.callback(|_| Msg::Noop);
        let router = RouteAgent::bridge(callback);
        Logout {
            fetch_task: None,
            link,
            router,
            storage,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.storage.remove("token");
        self.router.send(RouteRequest::ChangeRoute(Route::from("/login".to_string())));
        true
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {<></>}
    }
}
