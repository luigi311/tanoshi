use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender, SubmitEvent, InputData, Bridge, Bridged};
use yew_router::components::RouterAnchor;
use crate::app::AppRoute;
use stdweb::web::event::{IEvent, ClickEvent};
use yew::format::Json;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use serde::{Deserialize, Serialize};
use yew::services::StorageService;
use yew::services::storage::Area;
use stdweb::web::document;
use yew_router::prelude::*;
use yew_router::agent::RouteRequest;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub company: String,
    pub exp: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserResponse {
    pub claim: Option<Claims>,
    pub token: Option<String>,
    pub status: String,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
}

pub struct Login {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    router: Box<dyn Bridge<RouteAgent>>,
    storage: StorageService,
    user: User,
}

pub enum Msg {
    UsernameChange(InputData),
    PasswordChange(InputData),
    Submit(SubmitEvent),
    LoggedIn(UserResponse),
    Noop,
}

impl Component for Login {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local);
        let callback = link.callback(|_| Msg::Noop);
        let router = RouteAgent::bridge(callback);
        Login {
            fetch_task: None,
            link,
            router,
            storage,
            user: User {
                username: "".to_string(),
                password: "".to_string(),
            }
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::UsernameChange(e) => {
                self.user.username = e.value;
            }
            Msg::PasswordChange(e) => {
                self.user.password = e.value;
            }
            Msg::Submit(e) => {
                e.prevent_default();
                self.login();
            }
            Msg::LoggedIn(res) => {
                match res.token {
                    Some(token) => {
                        self.storage.store("token", Ok(token));
                        self.router.send(RouteRequest::ChangeRoute(Route::from("/".to_string())));
                    },
                    None => {return false;}
                }
            }
            Msg::Noop => {
                info!("noop");
                return false;
            },
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container">
                <div class="row">
                    <div class="col">
                        <img src="/assets/img/tanoshi.png"  class="rounded mx-auto d-block"/>
                    </div>
                </div>
                <div class="row">
                    <div class="col">
                        <form onsubmit=self.link.callback(|e| Msg::Submit(e))>
                            <div class="form-group">
                                <label for="inputEmail">{"Email address"}</label>
                                <input type="email" class="form-control" id="inputEmail"
                                 value=self.user.username.to_owned()
                                oninput=self.link.callback(|e| Msg::UsernameChange(e)) />
                            </div>
                            <div class="form-group">
                                <label for="inputPassword">{"Password"}</label>
                                <input type="password" class="form-control" id="inputPassword"
                                value=self.user.password.to_owned()
                                oninput=self.link.callback(|e| Msg::PasswordChange(e))/>
                            </div>
                            <button type="submit" class="btn btn-primary">{"Submit"}</button>
                        </form>
                    </div>
                </div>
            </div>
        }
    }
}

impl Login {
    fn login(&mut self) {
        let req = Request::post("/api/login")
            .header("Content-Type", "application/json")
            .body(Json(&self.user)).expect("failed to build request");

        let task = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Json<Result<UserResponse, anyhow::Error>>>| {
                if let (meta, Json(Ok(data))) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::LoggedIn(data);
                    }
                }
                Msg::Noop
            }));
        self.fetch_task = Some(task);
    }
}