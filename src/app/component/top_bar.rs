use crate::app::AppRoute;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

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
            <nav class="navbar navbar-expand-sm fixed-top navbar-light border-top-0" id="navbar" style="background-color: cornflowerblue;">
                <RouterAnchor<AppRoute> route=AppRoute::Home>
                    <div class="navbar-brand">{"Tanoshi"}</div>
                </RouterAnchor<AppRoute>>
                <div class="collapse navbar-collapse" id="navbarNav">
                    <ul class="navbar-nav">
                        <li class="nav-item"><a href="#" class="nav-link">{"Updates"}</a></li>
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
                        <li class="nav-item"><a href="#" class="nav-link">{"Settings"}</a></li>
                        <li class="nav-item">
                            <RouterAnchor<AppRoute> route=AppRoute::Logout>
                            <a href="#" class="nav-link">{"Logout"}</a>
                            </RouterAnchor<AppRoute>>
                        </li>
                    </ul>
                </div>
            </nav>
        }
    }
}
