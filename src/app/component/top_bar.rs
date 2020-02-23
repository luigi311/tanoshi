use yew::{Callback, ClickEvent, Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;
use yew::prelude::*;
use crate::app::AppRoute;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub title: String,
    pub thumbnail: String,
}

pub struct TopBar {
    link: ComponentLink<Self>,
}

pub enum Msg {
    Click
}

impl Component for TopBar {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        TopBar {
            link: link,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div class="navbar navbar-expand-lg navbar-light bg-light">
                <RouterAnchor<AppRoute> route=AppRoute::Home>
                    <div class="navbar-brand">{"Tanoshi"}</div>
                </RouterAnchor<AppRoute>>
                <div class="collapse navbar-collapse" id="navbarNav">
                    <ul class="navbar-nav">
                        <li class="nav-item"><a href="/updates" class="nav-link">{"Updates"}</a></li>
                        <li class="nav-item dropdown">
                            <a class="nav-link dropdown-toggle" href="#" id="navbarDropdown" role="button"
                            data-toggle="dropdown" aria-haspopup="true" aria-expanded="false">{"Catalogue"}</a>
                            <div class="dropdown-menu" aria-labelledby="navbarDropdown">
                                    <RouterAnchor<AppRoute> route=AppRoute::Source("mangasee".to_string())>
                                    <a class="dropdown-item">{"Mangasee"}</a>
                                    </RouterAnchor<AppRoute>>
                                    <RouterAnchor<AppRoute> route=AppRoute::Source("mangadex".to_string())>
                                    <a class="dropdown-item">{"Mangadex"}</a>
                                    </RouterAnchor<AppRoute>>
                                    <RouterAnchor<AppRoute> route=AppRoute::Source("mangaplus".to_string())>
                                    <a class="dropdown-item">{"Mangaplus"}</a></RouterAnchor<AppRoute>>
                            </div>
                        </li>
                        <li class="nav-item"><a href="/settings" class="nav-link">{"Settings"}</a></li>
                    </ul>
                </div>
            </div>
        }
    }
}