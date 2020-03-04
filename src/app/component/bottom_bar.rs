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
                   <img src="/assets/img/star-fill.svg" alt="" width="32" height="32" title="Favorites"/>
               </RouterAnchor<AppRoute>>
               <RouterAnchor<AppRoute> route=AppRoute::Source("mangasee".to_string())>
                   <img src="/assets/img/list.svg" alt="" width="32" height="32" title="Updates"/>
               </RouterAnchor<AppRoute>>
               <a href="#">
                   <img src="/assets/img/clock-fill.svg" alt="" width="32" height="32" title="Updates"/>
               </a>
               <a href="#">
                   <img src="/assets/img/gear-wide-connected.svg" alt="" width="32" height="32" title="Settings"/>
               </a>
               <a href="#">
                   <img src="/assets/img/person-fill.svg" alt="" width="32" height="32" title="Settings"/>
               </a>
           </div>
        }
    }
}
