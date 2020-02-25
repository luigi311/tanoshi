use yew::{Component, ComponentLink, html, Html, ShouldRender};
use yew_router::{router::Router, Switch};

use super::catalogue::Catalogue;
use super::chapter::Chapter;
use super::detail::Detail;
use super::home::Home;
use super::component::TopBar;

#[derive(Switch, Debug, Clone)]
pub enum AppRoute {
    #[to = "/catalogue/{source}/manga/{title}/chapter/{chapter}/page/{page}"]
    Chapter(String, String, String, usize),
    #[to = "/catalogue/{source}/manga/{title}"]
    Detail(String, String),
    #[to = "/catalogue/{source}"]
    Source(String),
    #[to = "/"]
    Home,
}

pub struct App {
}

pub enum Msg {
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        App {
        }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn view(&self) -> Html {
        html! {
        <>
            <TopBar />
            <Router<AppRoute, ()>
            render = Router::render(|switch: AppRoute| {
            match switch {
                AppRoute::Chapter(source, title, chapter, page) => html!{<Chapter source=source title=title chapter=chapter page=page/>},
                AppRoute::Detail(source, title) => html!{<Detail source=source title=title/>},
                AppRoute::Source(source) => html!{<Catalogue source=source/>},
                AppRoute::Home => html!{<Home/>},
            }
        })
    />
    </>
        }
    }
}
