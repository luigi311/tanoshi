use yew::{Callback, ClickEvent, Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;
use yew::prelude::*;
use crate::app::AppRoute;


#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub src: String,
}

pub struct Page {
    link: ComponentLink<Self>,
    src: String,
}

pub enum Msg {
    Click
}

impl Component for Page {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Page {
            link: link,
            src: props.src,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <img class="manga-page" src=self.src/>
        }
    }
}