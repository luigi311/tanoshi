use std::rc::Rc;

use dominator::{routing, text_signal};
use dominator::{clone, html, Dom};
use futures_signals::signal::Mutable;
use futures_signals::signal::SignalExt;
use wasm_bindgen::UnwrapThrowExt;

use crate::common::{Route, SettingCategory, events};
use crate::query::{user_login, user_register};
use crate::utils::AsyncLoader;

use super::Role;


pub struct Login {
    username: Mutable<String>,
    password: Mutable<String>,
    role: Mutable<Role>,
    pub loader: AsyncLoader,
}

impl Login {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            username: Mutable::new("".to_string()),
            password: Mutable::new("".to_string()),
            role: Mutable::new(Role::Reader),
            loader: AsyncLoader::new(),
        })
    }

    fn register(login: Rc<Self>) {
        login.loader.load(clone!(login => async move {
            let username = login.username.get_cloned();
            let password = login.password.get_cloned();
            let role = match login.role.get_cloned() {
                Role::Reader => user_register::Role::READER,
                Role::Admin => user_register::Role::ADMIN,
            };
            if user_register(username, password, role).await.is_ok() {
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
                    .children(&mut [
                        html!("div", {
                            .class([
                                "w-full",
                                "bg-gray-200",
                                "dark:bg-gray-700",
                                "rounded",
                                "p-1"
                            ])
                            .children(&mut [
                                html!("button", {
                                    .class("w-1/2")
                                    .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], login.role.signal_cloned().map(|x| match x {
                                        Role::Reader => true,
                                        Role::Admin => false,
                                    }))
                                    .text("Reader")
                                    .event(clone!(login => move |_: events::Click| login.role.set_neq(Role::Reader)))
                                }),
                                html!("button", {
                                    .class("w-1/2")
                                    .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], login.role.signal_cloned().map(|x| match x {
                                        Role::Reader => false,
                                        Role::Admin => true,
                                    }))
                                    .text("Admin")
                                    .event(clone!(login => move |_: events::Click| login.role.set_neq(Role::Admin)))
                                }),
                            ])
                        })
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
