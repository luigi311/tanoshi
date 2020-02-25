use serde::{Deserialize};
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};

#[derive(Deserialize, Debug)]
pub struct MangaModel {
    pub title: String,
    pub thumbnail_url: String,
}

#[derive(Clone, Properties)]
pub struct Props {
    pub source: String,
}

pub struct Home {
}

pub enum Msg {
}

impl Component for Home {
    type Message = Msg;
    type Properties = Props;

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        Home {
        }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn mounted(&mut self) -> ShouldRender {
        true
    }

    fn view(&self) -> Html {
        html! {
            <>
            </>
        }
    }
}
