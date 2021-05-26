use std::rc::Rc;

use dominator::routing;
use dominator::{Dom, clone, html};
use futures_signals::signal::Mutable;
use wasm_bindgen::UnwrapThrowExt;

use crate::query::user_login;
use crate::utils::AsyncLoader;
use crate::common::{Route, events};
use crate::utils::local_storage;

pub struct Login {
    username: Mutable<String>,
    password: Mutable<String>,
    loader: AsyncLoader,
}

impl Login {
    pub fn new() -> Rc<Self> {
        Rc::new(Self{
            username: Mutable::new("".to_string()),
            password: Mutable::new("".to_string()),
            loader: AsyncLoader::new()
        })
    }

    pub fn login(login: Rc<Self>) {
        login.loader.load(clone!(login => async move {
            let username = login.username.get_cloned();
            let password = login.password.get_cloned();
            if let Ok(token) = user_login(username, password).await {
                local_storage().set("token", &token).unwrap_throw();
                routing::go_to_url(&Route::Library.url());
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
                    .event_preventable(clone!(login => move |e: events::Input| {
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
                    .event_preventable(clone!(login => move |e: events::Input| {
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
                                "bg-accent",
                                "active:bg-accent-lighter",
                                "hover:bg-accent-lighter",
                                "outline-none",
                                "text-white",
                                "px-2",
                                "py-1",
                                "rounded"
                            ])
                            .text("Login")
                            .event_preventable(clone!(login => move |_: events::Click| {
                                Self::login(login.clone());
                            }))
                        })
                    ])
                })
            ])
        })
    }
}