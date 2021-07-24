use std::rc::Rc;

use dominator::{clone, html, Dom};
use dominator::{routing, with_node};
use futures_signals::signal::Mutable;
use futures_signals::signal::SignalExt;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;

use crate::common::{events, snackbar, Route};
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
                    snackbar::show(format!("Login failed: {}", e));
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
                    snackbar::show(format!("error: {:?}", e));
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
                    snackbar::show(format!("error check server status: {}", e));
                }
            }
        }));
    }

    pub fn render_topbar(login: Rc<Self>) -> Dom {
        html!("div", {
            .class("topbar")
        })
    }

    pub fn render_main(login: Rc<Self>) -> Dom {
        html!("div", {
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("max-width", "1024px")
            .style("margin", "auto")
            .style("padding", "0.5rem")
            .children(&mut [
                html!("img", {
                    .style("width", "8rem")
                    .style("height", "8rem")
                    .style("border-radius", "0.5rem")
                    .style("margin", "auto")
                    .attribute("src", "/icons/512.png")
                }),
                html!("div", {
                    .style("color", "white")
                    .style("background-color", "var(--primary-color)")
                    .style("border-radius", "0.5rem")
                    .style("padding", "0.25rem")
                    .visible_signal(login.server_status.signal_cloned().map(|x| {
                        if let Some(status) = x {
                            !status.activated
                        } else {
                            false
                        }
                    }))
                    .text("Server is not activated, activate by create an account")
                }),
                html!("form", {
                    .style("display", "flex")
                    .style("flex-direction", "column")
                    .event_preventable(|e: events::KeyDown| {
                        if e.key() == "enter" {
                            e.prevent_default();
                        }
                    })
                    .children(&mut [
                        html!("input" => HtmlInputElement, {
                            .attribute("type", "username")
                            .attribute("placeholder", "Username")
                            .property_signal("value", login.username.signal_cloned())
                            .with_node!(input => {
                                .event(clone!(login => move |_: events::Input| {
                                    login.username.set(input.value());
                                }))
                            })
                        }),
                        html!("input" => HtmlInputElement, {
                            .attribute("type", "password")
                            .attribute("placeholder", "Password")
                            .property_signal("value", login.password.signal_cloned())
                            .with_node!(input => {
                                .event(clone!(login => move |e: events::Input| {
                                    login.password.set(input.value());
                                }))
                            })
                        }),
                        html!("div", {
                            .style("display", "flex")
                            .style("justify-content", "flex-end")
                            .child_signal(login.server_status.signal_cloned().map(clone!(login => move |x| {
                                if x.map(|status| status.activated).unwrap_or(false) {
                                    Some(html!("button", {
                                        .text("Login")
                                        .event_preventable(clone!(login => move |e: events::Click| {
                                            e.prevent_default();
                                            Self::login(login.clone());
                                        }))
                                    }))
                                } else {
                                    Some(html!("button", {
                                        .text("Create Account")
                                        .event(clone!(login => move |_: events::Click| {
                                            Self::register(login.clone());
                                        }))
                                    }))
                                }
                            })))
                        })
                    ])
                })
            ])
        })
    }

    pub fn render(login: Rc<Self>) -> Dom {
        Self::fetch_server_status(login.clone());

        html!("div", {
            .class([
                "main",
            ])
            .children(&mut [
                Self::render_topbar(login.clone()),
                html!("div", {
                    .class("topbar-spacing")
                }),
                Self::render_main(login.clone()),
            ])
        })
    }
}
