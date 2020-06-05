use serde::{Deserialize, Serialize};

use yew::format::Json;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::{storage::Area, StorageService};
use yew::{
    html, Bridge, Bridged, Component, ComponentLink, Html, InputData, Properties, ShouldRender,
};

use yew_router::components::RouterAnchor;
use yew_router::{agent::RouteRequest, prelude::*};

use web_sys::{Event, HtmlElement};

use super::component::model::{Claims, User};
use crate::app::AppRoute;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserResponse {
    pub claim: Option<Claims>,
    pub token: Option<String>,
    pub status: String,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {}

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
    Submit(Event),
    LoggedIn(UserResponse),
    Noop,
}

impl Component for Login {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let callback = link.callback(|_| Msg::Noop);
        let router = RouteAgent::bridge(callback);
        Login {
            fetch_task: None,
            link,
            router,
            storage,
            user: User {
                username: "".to_string(),
                password: None,
                role: "".to_string(),
            },
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::UsernameChange(e) => {
                self.user.username = e.value;
            }
            Msg::PasswordChange(e) => {
                self.user.password = Some(e.value);
            }
            Msg::Submit(e) => {
                e.prevent_default();
                self.login();
            }
            Msg::LoggedIn(res) => match res.token {
                Some(token) => {
                    self.storage.store("token", Ok(token));
                    self.router
                        .send(RouteRequest::ChangeRoute(Route::from("/".to_string())));
                }
                None => {
                    return false;
                }
            },
            Msg::Noop => {
                return false;
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container mx-auto" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <div class="block fixed inset-x-0 top-0 z-50 bg-tachiyomi-blue safe-top z-50 shadow"></div>
                <div class="flex justify-center px-6 my-12">
                <div class="w-full xl:w-3/4 lg:w-11/12 flex">
                    <div class="w-full  bg-white p-5 rounded-lg lg:rounded-l-none">
                        <form class="px-8 pt-6 pb-8 mb-4 bg-white rounded" onsubmit=self.link.callback(|e| Msg::Submit(e))>
                            <div class="mb-4">
                                <label class="block mb-2 text-sm font-bold text-gray-700" for="username">
                                    {"Username"}
                                </label>
                                <input
                                    class="w-full px-3 py-2 text-sm leading-tight text-gray-700 border rounded shadow appearance-none focus:outline-none focus:shadow-outline"
                                    id="username"
                                    type="text"
                                    value=self.user.username.to_owned()
                                    oninput=self.link.callback(|e| Msg::UsernameChange(e))
                                />
                            </div>
                            <div class="mb-4">
                                <label class="block mb-2 text-sm font-bold text-gray-700" for="password">
                                    {"Password"}
                                </label>
                                <input
                                    class="w-full px-3 py-2 mb-3 text-sm leading-tight text-gray-700 border rounded shadow appearance-none focus:outline-none focus:shadow-outline"
                                    id="password"
                                    type="password"
                                    value=self.user.password.clone().unwrap_or("".to_string()).to_owned()
                                    oninput=self.link.callback(|e| Msg::PasswordChange(e))
                                />
                            </div>
                            <div class="mb-6 text-center">
                                <button
                                    class="w-full px-4 py-2 font-bold text-white bg-blue-500 rounded-full hover:bg-blue-700 focus:outline-none focus:shadow-outline"
                                    type="button"
                                >
                                    {"Sign In"}
                                </button>
                            </div>
                        </form>
                    </div>
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
            .body(Json(&self.user))
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<UserResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::LoggedIn(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
}
