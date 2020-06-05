use yew::services::storage::Area;
use yew::services::{fetch::FetchTask, FetchService, StorageService};
use yew::{
    format::{Nothing, Text},
    html, Bridge, Bridged, Component, ComponentLink, Html, ShouldRender,
};
use yew_router::agent::RouteRequest;
use yew_router::prelude::{Route, RouteAgent};
use yew_router::{router::Router, Switch};

use super::browse::{self, Browse, BrowseRoute};
use super::chapter::Chapter;
use super::job;
use super::login::Login;
use super::logout::Logout;
use http::{Request, Response};

use yew::prelude::*;

#[derive(Switch, Debug, Clone)]
pub enum AppRoute {
    #[to = "/chapter/{chapter_id}/page/{page}"]
    Chapter(i32, usize),
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
    worker: Box<dyn Bridge<job::Worker>>,
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

        let worker_callback = link.callback(|_| Msg::Noop);
        let worker = job::Worker::bridge(worker_callback);

        App {
            link,
            storage,
            router,
            route: "/".to_string(),
            fetch_task: None,
            worker,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            if let Ok(token) = self.storage.restore("token") {
                self.validate_token(token);
            } else {
                self.router
                    .send(RouteRequest::ChangeRoute(Route::from("/login".to_string())));
            }
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
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
                    AppRoute::Chapter(chapter_id, page) => html!{<Chapter chapter_id=chapter_id page=page/>},
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
            self.link.callback(|response: Response<Text>| {
                let (meta, _res) = response.into_parts();
                let status = meta.status;
                if status == http::StatusCode::UNAUTHORIZED {
                    return Msg::TokenInvalidorExpired;
                }
                Msg::Noop
            }),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
}
