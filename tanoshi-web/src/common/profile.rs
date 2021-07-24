use std::rc::Rc;

use dominator::{routing, with_node};
use dominator::{clone, html, Dom};
use futures_signals::signal::Mutable;
use futures_signals::signal::SignalExt;
use web_sys::HtmlInputElement;

use crate::common::{Route, SettingCategory, events, snackbar};
use crate::query;
use crate::utils::AsyncLoader;

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
                    snackbar::show(format!("change password error: {}", e));
                }
            };
        }));
    }

    pub fn render(profile: Rc<Self>) -> Dom {
        html!("form", {
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("max-width", "1024px")
            .style("margin-left", "auto")
            .style("margin-right", "auto")
            .style("border-radius", "0.5rem")
            .children(&mut [
                html!("input" => HtmlInputElement, {
                    .attribute("type", "password")
                    .attribute("placeholder", "Current Password")
                    .property_signal("value", profile.old_password.signal_cloned())
                    .with_node!(input => {
                        .event(clone!(profile => move |_: events::Input| {
                            profile.old_password.set(input.value());
                        }))
                    })
                }),
                html!("span", {
                    .visible_signal(profile.confirm_password.signal_cloned().map(clone!(profile => move |x| {
                        if x != profile.new_password.get_cloned() {
                            true
                        } else {
                            false
                        }
                    })))
                    .style("font-size", "small")
                    .style("margin-left", "0.5rem")
                    .style("margin-right", "0.5rem")
                    .style("color", "red")
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
                    .style("font-size", "small")
                    .style("margin-left", "0.5rem")
                    .style("margin-right", "0.5rem")
                    .style("color", "red")
                    .text("Minimum password length is 8")
                }),
                html!("input" => HtmlInputElement, {
                    .class_signal("error", profile.confirm_password.signal_cloned().map(clone!(profile => move |x| {
                        if x != profile.new_password.get_cloned() {
                            true
                        } else {
                            false
                        }
                    })))
                    .attribute("type", "password")
                    .attribute("placeholder", "New Password")
                    .property_signal("value", profile.new_password.signal_cloned())
                    .with_node!(input => {
                        .event(clone!(profile => move |_: events::Input| {
                            profile.new_password.set(input.value());
                        }))
                    })
                }),
                html!("input" => HtmlInputElement, {
                    .class_signal("error", profile.confirm_password.signal_cloned().map(clone!(profile => move |x| {
                        if x != profile.new_password.get_cloned() {
                            true
                        } else {
                            false
                        }
                    })))
                    .attribute("type", "password")
                    .attribute("placeholder", "Confirm Password")
                    .property_signal("value", profile.confirm_password.signal_cloned())
                    .with_node!(input => {
                        .event(clone!(profile => move |_: events::Input| {
                            profile.confirm_password.set(input.value());
                        }))
                    })
                }),
                html!("div", {
                    .style("display", "flex")
                    .style("justify-content", "flex-end")
                    .style("margin", "0.5rem")
                    .children(&mut [
                        html!("input", {
                            .attribute("type", "submit")
                            .attribute_signal("disabled", profile.confirm_password.signal_cloned().map(clone!(profile => move |x| {
                                if x != profile.new_password.get_cloned() || x.len() < 8 {
                                    Some("true")
                                } else {
                                    None
                                }
                            })))
                            .text("Submit")
                            .event_preventable(clone!(profile => move |e: events::Click| {
                                e.prevent_default();
                                Self::change_password(profile.clone());
                            }))
                        })
                    ])
                })
            ])
        })
    }
}
