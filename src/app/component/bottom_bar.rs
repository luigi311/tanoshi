use crate::app::AppRoute;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {}

pub struct BottomBar {}

pub enum Msg {}

impl Component for BottomBar {
    type Message = Msg;
    type Properties = Props;

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        BottomBar {}
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
           <div class="bottom-bar">
               <RouterAnchor<AppRoute> route=AppRoute::Home>
                   <div>
                        <img class="icon" src="/assets/img/star-fill.svg" alt="" width="32" height="32" title="Favorites"/>
                        <p>{"Favorites"}</p>
                   </div>
               </RouterAnchor<AppRoute>>
               <RouterAnchor<AppRoute> route=AppRoute::Source("mangasee".to_string())>
                   <div>
                        <img class="icon" src="/assets/img/list.svg" alt="" width="32" height="32" title="Catalogue"/>
                        <p>{"Catalogue"}</p>
                   </div>
               </RouterAnchor<AppRoute>>
               <a href="#">
                   <div>
                        <img class="icon" src="/assets/img/clock-fill.svg" alt="" width="32" height="32" title="Updates"/>
                        <p>{"Updates"}</p>
                   </div>
               </a>
               <a href="#">
                    <div>
                        <img class="icon" src="/assets/img/gear-wide-connected.svg" alt="" width="32" height="32" title="Settings"/>
                        <p>{"Settings"}</p>
                    </div>
               </a>
               <a href="#">
                    <div>
                        <img class="icon" src="/assets/img/person-fill.svg" alt="" width="32" height="32" title="Account"/>
                        <p>{"Account"}</p>
                    </div>
               </a>
           </div>
        }
    }
}
