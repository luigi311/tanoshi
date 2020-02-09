use yew::{Callback, ClickEvent, Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew::prelude::*;


#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub title: String,
    pub thumbnail: String,
}

pub struct Manga {
    link: ComponentLink<Self>,
    title: String,
    thumbnail: String,
}

pub enum Msg {
    Click
}

impl Component for Manga {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Manga {
            link: link,
            title: props.title,
            thumbnail: props.thumbnail,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let title = self.title.to_owned();
        let thumbnail = self.thumbnail.to_owned();
        html! {
            <div class="pure-u-lg-1-5">
                <div class="manga-cover-container">
                    <img class="manga-cover" src=thumbnail />
                    <div class="manga-cover-title">{title}</div>
                </div>
            </div>
        }
    }
}