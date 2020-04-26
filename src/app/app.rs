use yew::services::storage::Area;
use yew::services::{fetch::FetchTask, StorageService, FetchService};
use yew::{html, Bridge, Bridged, Component, ComponentLink, Html, NodeRef, ShouldRender, format::{Text, Nothing}};
use yew_router::agent::RouteRequest;
use yew_router::prelude::{Route, RouteAgent};
use yew_router::{router::Router, Switch};

use web_sys::HtmlElement;

use super::browse::{self, Browse, BrowseRoute};
use super::chapter::Chapter;
use super::login::Login;
use super::logout::Logout;
use http::{Response, Request};

#[derive(Switch, Debug, Clone)]
pub enum AppRoute {
    #[to = "/catalogue/{source}/manga/{title}/chapter/{chapter}/page/{page}"]
    Chapter(String, String, String, usize),
    #[to = "/login"]
    Login,
    #[to = "/logout"]
    Logout,
    #[to = "{*:path}"]
    Browse(BrowseRoute),
}

pub struct App {
    link: ComponentLink<Self>,
    storage: StorageService,
    router: Box<dyn Bridge<RouteAgent>>,
    route: String,
    fetch_task: Option<FetchTask>,
}

pub enum Msg {
    RouterCallback(Route),
    TokenInvalidorExpired,
    Noop,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let callback = link.callback(|route| Msg::RouterCallback(route));
        let router = RouteAgent::bridge(callback);
        App {
            link,
            storage,
            router,
            route: "/".to_string(),
            fetch_task: None,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        info!("change");
        true
    }

    fn mounted(&mut self) -> ShouldRender {
        if let Ok(token) = self.storage.restore("token") {
            self.validate_token(token);
        } else {
            self.router
                .send(RouteRequest::ChangeRoute(Route::from("/login".to_string())));
        }
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        info!("update");
        match msg {
            Msg::RouterCallback(route) => {
                self.route = route.route;
            }
            Msg::TokenInvalidorExpired => {
                self.router
                .send(RouteRequest::ChangeRoute(Route::from("/login".to_string())));
            }
            Msg::Noop => {
                return false;
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="w-full h-screen">
                <Router<AppRoute, ()>
                render = Router::render(|switch: AppRoute| {
                match switch {
                    AppRoute::Chapter(source, title, chapter, page) => html!{<Chapter source=source title=title chapter=chapter page=page/>},
                    AppRoute::Login => html!{<Login />},
                    AppRoute::Logout => html!{<Logout />},
                    AppRoute::Browse(route) => {
                        let route: browse::Props = route.into();
                        html!{<Browse with route/>}
                    },
                }}) />
            </div>
        }
    }
}

impl App {
    fn validate_token(&mut self, token: String) {
        let req = Request::get("/api/validate")
            .header("Authorization", token)
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Text>| {
                    let (meta, res) = response.into_parts();
                    let status = meta.status;
                    if status == http::StatusCode::UNAUTHORIZED {
                        return Msg::TokenInvalidorExpired
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
}
