use std::rc::Rc;

use dominator::{Dom, clone, html, with_node};
use dominator::{routing, text_signal};
use futures_signals::signal::Mutable;
use web_sys::HtmlInputElement;
use futures_signals::signal::SignalExt;
use wasm_bindgen::UnwrapThrowExt;

use crate::common::{events, Route, SettingCategory};
use crate::query;
use crate::utils::AsyncLoader;

pub struct Login {
    username: Mutable<String>,
    password: Mutable<String>,
    is_admin: Mutable<bool>,
    pub loader: AsyncLoader,
}

impl Login {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            username: Mutable::new("".to_string()),
            password: Mutable::new("".to_string()),
            is_admin: Mutable::new(false),
            loader: AsyncLoader::new(),
        })
    }

    fn register(login: Rc<Self>) {
        login.loader.load(clone!(login => async move {
            let username = login.username.get_cloned();
            let password = login.password.get_cloned();
            let is_admin = login.is_admin.get();
            if query::user_register(username, password, is_admin).await.is_ok() {
                login.username.set("".to_string());
                login.password.set("".to_string());

                routing::go_to_url(Route::Settings(SettingCategory::Users).url().as_str());
            }
        }));
    }

    pub fn render(login: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "flex",
                "flex-col",
                "max-w-lg",
                "mx-auto",
                "mt-safe-top",
                "bg-white",
                "dark:bg-gray-800",
                "shadow",
                "dark:shadow-none",
                "rounded",
                "p-2"
            ])
            .children(&mut [
                html!("input", {
                    .class([
                        "m-2",
                        "p-1",
                        "outline-none",
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
                        "outline-none",
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
                        "items-center",
                        "p-2"
                    ])
                    .children(&mut [
                        html!("input" => HtmlInputElement, {
                            .attribute("type", "checkbox")
                            .with_node!(element => {
                                .event(clone!(login => move |_: events::Change| {
                                    login.is_admin.set_neq(element.checked());
                                }))
                            })
                        }),
                        html!("div", {
                            .class("mx-2")
                            .text("Admin")
                        }),
                    ])
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
                                "bg-accent",
                                "active:bg-accent-lighter",
                                "hover:bg-accent-lighter",
                                "outline-none",
                                "text-white",
                                "px-2",
                                "py-1",
                                "rounded"
                            ])
                            .text("Create")
                            .event(clone!(login => move |_: events::Click| {
                                Self::register(login.clone());
                            }))
                        })
                    ])
                })
            ])
        })
    }
}
