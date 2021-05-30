use std::rc::Rc;

use dominator::{routing, text_signal};
use dominator::{clone, html, Dom};
use futures_signals::signal::Mutable;
use futures_signals::signal::SignalExt;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::console::info;

use crate::common::{events, Route, SettingCategory};
use crate::query;
use crate::utils::AsyncLoader;

use super::Role;

pub struct Profile {
    old_password: Mutable<String>,
    new_password: Mutable<String>,
    confirm_password: Mutable<String>,
    pub loader: AsyncLoader,
}

impl Profile {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            old_password: Mutable::new("".to_string()),
            new_password: Mutable::new("".to_string()),
            confirm_password: Mutable::new("".to_string()),
            loader: AsyncLoader::new(),
        })
    }

    fn change_password(profile: Rc<Self>) {
        profile.loader.load(clone!(profile => async move {
            let old_password = profile.old_password.get_cloned();
            let new_password = profile.new_password.get_cloned();
            match query::change_password(old_password, new_password).await {
                Ok(_) => {
                    routing::go_to_url(Route::Settings(SettingCategory::None).url().as_str());
                },
                Err(e) => {
                    log::error!("{}", e);
                }
            };
        }));
    }

    pub fn render(profile: Rc<Self>) -> Dom {
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
                    .attribute("type", "password")
                    .attribute("placeholder", "Current Password")
                    .property_signal("value", profile.old_password.signal_cloned())
                    .event(clone!(profile => move |e: events::Input| {
                        profile.old_password.set(e.value().unwrap_or("".to_string()));
                    }))
                }),
                html!("span", {
                    .visible_signal(profile.confirm_password.signal_cloned().map(clone!(profile => move |x| {
                        if x != profile.new_password.get_cloned() {
                            true
                        } else {
                            false
                        }
                    })))
                    .class([
                        "text-sm",
                        "mx-2",
                        "text-red-500"
                    ])
                    .text("Password do not match")
                }),
                
                html!("span", {
                    .visible_signal(profile.new_password.signal_cloned().map(|x| {
                        if x.len() < 8 {
                            true
                        } else {
                            false
                        }
                    }))
                    .class([
                        "text-sm",
                        "mx-2",
                        "text-red-500"
                    ])
                    .text("Minimum password length is 8")
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
                    .class_signal(["ring-1", "ring-red-500"], profile.confirm_password.signal_cloned().map(clone!(profile => move |x| {
                        if x != profile.new_password.get_cloned() {
                            true
                        } else {
                            false
                        }
                    })))
                    .attribute("type", "password")
                    .attribute("placeholder", "New Password")
                    .property_signal("value", profile.new_password.signal_cloned())
                    .event(clone!(profile => move |e: events::Input| {
                        profile.new_password.set(e.value().unwrap_or("".to_string()));
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
                    .class_signal(["ring-1", "ring-red-500"], profile.confirm_password.signal_cloned().map(clone!(profile => move |x| {
                        if x != profile.new_password.get_cloned() {
                            true
                        } else {
                            false
                        }
                    })))
                    .attribute("type", "password")
                    .attribute("placeholder", "Confirm Password")
                    .property_signal("value", profile.confirm_password.signal_cloned())
                    .event(clone!(profile => move |e: events::Input| {
                        profile.confirm_password.set(e.value().unwrap_or("".to_string()));
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
                            .attribute_signal("disabled", profile.confirm_password.signal_cloned().map(clone!(profile => move |x| {
                                if x != profile.new_password.get_cloned() || x.len() < 8 {
                                    Some("true")
                                } else {
                                    None
                                }
                            })))
                            .text("Submit")
                            .event(clone!(profile => move |_: events::Click| {
                                Self::change_password(profile.clone());
                            }))
                        })
                    ])
                })
            ])
        })
    }
}
