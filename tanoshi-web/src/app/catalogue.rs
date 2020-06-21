use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use yew_router::Switch;

use super::select::Select;
use super::source::Source;

#[derive(Switch, PartialEq, Debug, Clone)]
pub enum CatalogueRoute {
    #[to = "/{source_id}"]
    Source(i32),
    #[to = "/!"]
    Select,
}

#[derive(Clone, Properties)]
pub struct Props {
    pub route: CatalogueRoute,
}

pub struct Catalogue {
    route: CatalogueRoute,
}

pub enum Msg {}

impl Component for Catalogue {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Catalogue { route: props.route }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.route != props.route {
            self.route = props.route;
            return true;
        }
        false
    }

    fn view(&self) -> Html {
        html! {
            match &self.route {
                CatalogueRoute::Source(source_id) => {
                    html! {<Source source_id=source_id />}
                },
                CatalogueRoute::Select => {
                    html! {<Select />}
                }
            }
        }
    }
}
