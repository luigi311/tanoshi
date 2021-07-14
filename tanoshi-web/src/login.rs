use std::rc::Rc;

use dominator::{clone, html, Dom};
use dominator::{routing, text_signal};
use futures_signals::signal::Mutable;
use futures_signals::signal::SignalExt;
use wasm_bindgen::UnwrapThrowExt;

use crate::common::{events, Route};
use crate::query;
use crate::utils::local_storage;
use crate::utils::AsyncLoader;

#[derive(Debug, Clone)]
struct ServerStatus {
    activated: bool,
    version: String,
}

pub struct Login {
    username: Mutable<String>,
    password: Mutable<String>,
    server_status: Mutable<Option<ServerStatus>>,
    loader: AsyncLoader,
}

impl Login {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            username: Mutable::new("".to_string()),
            password: Mutable::new("".to_string()),
            server_status: Mutable::new(None),
            loader: AsyncLoader::new(),
        })
    }

    pub fn login(login: Rc<Self>) {
        login.loader.load(clone!(login => async move {
            let username = login.username.get_cloned();
            let password = login.password.get_cloned();
            match query::user_login(username, password).await {
                Ok(token) => {
                    local_storage().set("token", &token).unwrap_throw();
                    routing::go_to_url(&Route::Library.url());
                }
                Err(e) => {
                    error!("Login failed: {}", e);
                }
            }
        }));
    }

    pub fn register(login: Rc<Self>) {
        login.loader.load(clone!(login => async move {
            let username = login.username.get_cloned();
            let password = login.password.get_cloned();
            match query::user_register(username, password, true).await {
                Ok(_) => {
                    login.username.set("".to_string());
                    login.password.set("".to_string());
                    Self::fetch_server_status(login.clone());
                }
                Err(e) => {
                    error!("error: {:?}", e);
                }
            }
        }));
    }

    pub fn fetch_server_status(login: Rc<Self>) {
        login.loader.load(clone!(login => async move {
            match query::server_status().await {
                Ok(server_status) => {
                login.server_status.set(Some(ServerStatus{
                    activated: server_status.activated,
                    version: server_status.version,
                }));
                }
                Err(e) => {
                    error!("error check server status: {}", e);
                }
            }
        }));
    }

    pub fn render(login: Rc<Self>) -> Dom {
        Self::fetch_server_status(login.clone());

        html!("div", {
            .class([
                "flex",
                "flex-col",
                "max-w-lg",
                "mx-auto",
                "mt-safe-top"
            ])
            .children(&mut [
                html!("img", {
                    .class([
                        "w-32",
                        "h-32",
                        "mx-auto",
                        "rounded"
                    ])
                    .attribute("src", "/icons/512.png")
                }),
                html!("div", {
                    .class([
                        "text-white",
                        "bg-accent",
                        "rounded",
                        "m-2",
                        "px-2",
                        "py-1"
                    ])
                    .visible_signal(login.server_status.signal_cloned().map(|x| {
                        if let Some(status) = x {
                            !status.activated
                        } else {
                            false
                        }
                    }))
                    .text("Server is not activated, activate by create an account")
                }),
                html!("input", {
                    .class([
                        "m-2",
                        "p-1",
                        "focus:outline-none",
                        "rounded",
                        "bg-white",
                        "dark:bg-gray-900",
                        "text-black",
                        "dark:text-white"
                    ])
                    .attribute("type", "username")
                    .attribute("placeholder", "Username")
                    .property_signal("value", login.username.signal_cloned())
                    .event(clone!(login => move |e: events::Input| {
                        login.username.set(e.value().unwrap_or("".to_string()));
                    }))
                }),
                html!("input", {
                    .class([
                        "m-2",
                        "p-1",
                        "focus:outline-none",
                        "rounded",
                        "bg-white",
                        "dark:bg-gray-900",
                        "text-black",
                        "dark:text-white"
                    ])
                    .attribute("type", "password")
                    .attribute("placeholder", "Password")
                    .property_signal("value", login.password.signal_cloned())
                    .event(clone!(login => move |e: events::Input| {
                        login.password.set(e.value().unwrap_or("".to_string()));
                    }))
                }),
                html!("div", {
                    .class([
                        "flex",
                        "justify-end",
                        "m-2"
                    ])
                    .children(&mut [
                        html!("button", {
                            .class([
                                "mx-2",
                                "focus:outline-none",
                                "hover:underline",
                                "active:underline",
                                "text-black",
                                "dark:text-white",
                                "focus:outline-none"
                            ])
                            .visible_signal(login.server_status.signal_cloned().map(|x| {
                                if let Some(status) = x {
                                    !status.activated
                                } else {
                                    false
                                }
                            }))
                            .text("Create Account")
                            .event(clone!(login => move |_: events::Click| {
                                Self::register(login.clone());
                            }))
                        }),
                        html!("button", {
                            .class([
                                "bg-accent",
                                "active:bg-accent-lighter",
                                "hover:bg-accent-lighter",
                                "focus:outline-none",
                                "text-white",
                                "px-2",
                                "py-1",
                                "rounded",
                                "focus:outline-none"
                            ])
                            .text("Login")
                            .event(clone!(login => move |_: events::Click| {
                                Self::login(login.clone());
                            }))
                        })
                    ])
                })
            ])
        })
    }
}
