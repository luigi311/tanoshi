use yew_router::{router::Router, Switch};
use serde::{Deserialize, Serialize};
use yew::{Component, ComponentLink, html, Html, ShouldRender};
use yew::format::{Json, Nothing, Text};
use yew::html::{ChildrenRenderer, NodeRef, Properties};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

use super::catalogue::Catalogue;
use super::detail::Detail;

#[derive(Switch, Debug, Clone)]
pub enum AppRoute {
    #[to = "/catalogue/{source}/manga/{title}"]
    Detail(String, String),
    #[to = "/catalogue/{source}"]
    Source(String),
}

pub struct App {
    link: ComponentLink<Self>,
}

pub enum Msg {
    Noop,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        App {
            link,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <Router<AppRoute, ()>
            render = Router::render(|switch: AppRoute| {
            match switch {
                AppRoute::Detail(source, title) => html!{<Detail source=source title=title/>},
                AppRoute::Source(source) => html!{<Catalogue source=source/>},
            }
        })
    />
        }
    }
}
