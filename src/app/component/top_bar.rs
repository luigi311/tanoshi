use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use crate::app::AppRoute;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {}

pub struct TopBar {}

pub enum Msg {}

impl Component for TopBar {
    type Message = Msg;
    type Properties = Props;

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        TopBar {}
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <nav class="navbar navbar-expand-sm fixed-top navbar-light border-top-0 bg-dark" id="navbar" >
                <RouterAnchor<AppRoute> route=AppRoute::Home>
                    <div class="text-light">{"Home"}</div>
                </RouterAnchor<AppRoute>>
                <div class="collapse navbar-collapse" id="navbarNav">
                    <ul class="navbar-nav">
                        <li class="nav-item"><a href="#" class="nav-link">{"Updates"}</a></li>
                        <li class="nav-item">
                           <RouterAnchor<AppRoute> route=AppRoute::Source("mangasee".to_string())>
                           {"Catalogue"}
                           </RouterAnchor<AppRoute>>
                        </li>
                        <li class="nav-item"><a href="#" class="nav-item">{"Settings"}</a></li>
                        <li class="nav-item">
                            <RouterAnchor<AppRoute> route=AppRoute::Logout>
                            {"Logout"}
                            </RouterAnchor<AppRoute>>
                        </li>
                    </ul>
                </div>
            </nav>
        }
    }
}
