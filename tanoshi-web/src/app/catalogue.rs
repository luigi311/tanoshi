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
    link: ComponentLink<Self>,
    route: CatalogueRoute,
}

pub enum Msg {
    Noop,
}

impl Component for Catalogue {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Catalogue {
            link,
            route: props.route,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Noop => {}
        }
        true
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
