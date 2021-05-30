use crate::{
    query,
    common::{events, ReaderSettings, Route, SettingCategory, Login, Profile, Source, User},
    utils::AsyncLoader
};
use dominator::svg;
use dominator::{clone, html, link, routing, Dom};
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::MutableVec,
    signal_vec::SignalVecExt,
};
use std::rc::Rc;
use web_sys::window;

pub struct Settings {
    page: Mutable<SettingCategory>,
    installed_sources: MutableVec<Source>,
    available_sources: MutableVec<Source>,
    me: Mutable<Option<User>>,
    users: MutableVec<User>,
    reader_settings: Rc<ReaderSettings>,
    loader: AsyncLoader,
}

impl Settings {
    pub fn new() -> Rc<Self> {
        return Rc::new(Settings {
            page: Mutable::new(SettingCategory::None),
            installed_sources: MutableVec::new(),
            available_sources: MutableVec::new(),
            me: Mutable::new(None),
            users: MutableVec::new(),
            reader_settings: ReaderSettings::new(true, false),
            loader: AsyncLoader::new(),
        });
    }

    fn fetch_sources(settings: Rc<Self>) {
        settings.loader.load(clone!(settings => async move {
            match query::fetch_all_sources().await {
                Ok(result) => {
                    settings.installed_sources.lock_mut().replace_cloned(result.installed_sources.iter().map(|s| Source {
                        id: s.id,
                        name: s.name.clone(),
                        version: s.version.clone(),
                        icon: s.icon.clone(),
                        need_login: s.need_login,
                        has_update: s.has_update,
                    }).collect());

                    settings.available_sources.lock_mut().replace_cloned(result.available_sources.iter().map(|s| Source {
                        id: s.id,
                        name: s.name.clone(),
                        version: s.version.clone(),
                        icon: s.icon.clone(),
                        need_login: s.need_login,
                        has_update: s.has_update,
                    }).collect());                    
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    fn fetch_user_list(settings: Rc<Self>) {
        settings.loader.load(clone!(settings => async move {
            match query::fetch_users().await {
                Ok(result) => {
                    settings.users.lock_mut().replace_cloned(result.iter().map(|u| User{
                        id: u.id,
                        username: u.username.clone(),
                        is_admin: u.is_admin,
                    }).collect());
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    fn fetch_me(settings: Rc<Self>) {
        settings.loader.load(clone!(settings => async move {
            match query::fetch_me().await {
                Ok(result) => {
                    settings.me.set(Some(User{
                        id: result.id,
                        username: result.username,
                        is_admin: result.is_admin,
                    }))
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    fn install_source(settings: Rc<Self>, id: i64) {
        settings.loader.load(clone!(settings => async move {
            match query::install_source(id).await {
                Ok(_) => {},
                Err(err) => {
                    log::error!("{}", err);
                    return;
                }
            }

            match query::fetch_all_sources().await {
                Ok(result) => {
                    settings.installed_sources.lock_mut().replace_cloned(result.installed_sources.iter().map(|s| Source {
                        id: s.id,
                        name: s.name.clone(),
                        version: s.version.clone(),
                        icon: s.icon.clone(),
                        need_login: s.need_login,
                        has_update: s.has_update,
                    }).collect());

                    settings.available_sources.lock_mut().replace_cloned(result.available_sources.iter().map(|s| Source {
                        id: s.id,
                        name: s.name.clone(),
                        version: s.version.clone(),
                        icon: s.icon.clone(),
                        need_login: s.need_login,
                        has_update: s.has_update,
                    }).collect());
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    fn update_source(settings: Rc<Self>, id: i64) {
        settings.loader.load(clone!(settings => async move {
            match query::update_source(id).await {
                Ok(_) => {},
                Err(err) => {
                    log::error!("{}", err);
                    return;
                }
            }

            match query::fetch_all_sources().await {
                Ok(result) => {
                    settings.installed_sources.lock_mut().replace_cloned(result.installed_sources.iter().map(|s| Source {
                        id: s.id,
                        name: s.name.clone(),
                        version: s.version.clone(),
                        icon: s.icon.clone(),
                        need_login: s.need_login,
                        has_update: s.has_update,
                    }).collect());

                    settings.available_sources.lock_mut().replace_cloned(result.available_sources.iter().map(|s| Source {
                        id: s.id,
                        name: s.name.clone(),
                        version: s.version.clone(),
                        icon: s.icon.clone(),
                        need_login: s.need_login,
                        has_update: s.has_update,
                    }).collect());
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    fn uninstall_source(settings: Rc<Self>, id: i64) {
        settings.loader.load(async move {
            match query::uninstall_source(id).await {
                Ok(_) => {
                    routing::go_to_url(&Route::Settings(SettingCategory::Source(0)).url());
                },
                Err(err) => {
                    log::error!("{}", err);
                    return;
                }
            }
        });
    }

    pub fn render_topbar(settings: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "pl-2",
                "pr-2",
                "pb-2",
                "flex",
                "justify-between",
                "fixed",
                "left-0",
                "xl:left-48",
                "right-0",
                "top-0",
                "z-40",
                "bg-accent",
                "dark:bg-gray-900",
                "border-b",
                "border-accent-darker",
                "dark:border-gray-800",
                "text-gray-50",
                "pt-safe-top"
            ])
            .children(&mut [
                html!("button", {
                    .class([
                        "block",
                        "xl:hidden"
                    ])
                    .text("Close")
                    .event(|_: events::Click| {
                        let history = window().unwrap().history().unwrap();
                        if history.length().unwrap() > 1 {
                            let _ = history.back();
                        } else {
                            routing::go_to_url("/");
                        }
                    })
                }),
                html!("span", {
                    .class(["text-gray-300", "truncate", "mx-auto"])
                    .text_signal(settings.page.signal_cloned().map(|x|
                        match x {
                            SettingCategory::None => "Settings",
                            SettingCategory::Reader => "Reader",
                            SettingCategory::Source(_) => "Sources",
                            SettingCategory::Users => "Users",
                            SettingCategory::CreateUser => "Create User",
                            SettingCategory::User => "User",
                        }
                    ))
                }),
                html!("div", {
                    .child_signal(settings.page.signal_cloned().map(move |page| {
                        match page {
                            SettingCategory::Reader => {
                                Some(ReaderSettings::render_apply_button(settings.reader_settings.clone()))
                            }
                            SettingCategory::Users => {
                                Some(link!(Route::Settings(SettingCategory::CreateUser).url(), {
                                    .text("Create User")
                                }))
                            }
                            _ => {
                                Some(
                                    html!("span", {
                                        .class("text-gray-300")
                                        .text("")
                                    }))
                            }
                        }
                    }))
                }),
            ])
        })
    }

    pub fn render_categories(settings: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "w-full",
                "mx-auto",
                "flex",
                "flex-col",
                "justify-start",
                "rounded",
                "bg-white",
                "dark:bg-gray-900",
                "shadow",
                "dark:shadow-none",
                "divide-y",
                "divide-gray-200",
                "dark:divide-gray-800",
                "px-2",
                "text-gray-900",
                "dark:text-gray-100",
            ])
            .children(&mut [
                link!(Route::Settings(SettingCategory::Reader).url(), {
                    .class([
                        "p-2",
                        "text-left"
                    ])
                    .text("Reader")
                }),
                link!(Route::Settings(SettingCategory::Source(0)).url(), {
                    .class([
                        "p-2",
                        "text-left"
                    ])
                    .text("Source")
                })
            ])
            .child_signal(settings.me.signal_cloned().map(|me| {
                if let Some(me) = me {
                    if me.is_admin {
                        Some(link!(Route::Settings(SettingCategory::Users).url(), {
                            .class([
                                "p-2",
                                "text-left"
                            ])
                            .text("Users")
                        }))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }))
        })
    }

    fn render_source_list(settings: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "w-full",
                "mx-auto"
            ])
            .children(&mut [
                html!("h1", {
                    .class([
                        "text-gray-900",
                        "dark:text-gray-100",
                    ])
                    .text("Installed")
                }),
                html!("div", {
                    .class([
                        "rounded",
                        "bg-white",
                        "dark:bg-gray-900",
                        "shadow",
                        "dark:shadow-none",
                        "divide-y",
                        "divide-gray-200",
                        "dark:divide-gray-800",
                        "px-2"
                    ])
                    .children_signal_vec(settings.installed_sources.signal_vec_cloned().map(clone!(settings => move |x|
                        html!("div", {
                            .class([
                                "p-2",
                                "flex",
                                "justify-between"
                            ])
                            .children(&mut [
                                link!(Route::Settings(SettingCategory::Source(x.id)).url(), {
                                    .class("flex")
                                    .children(&mut [
                                        html!("img", {
                                            .class([
                                                "w-10",
                                                "h-10",
                                                "mr-2"
                                            ])
                                            .attribute("src", &["data:image/png;base64,", &x.icon].join(" "))
                                        }),
                                        html!("div", {
                                            .children(&mut [
                                                html!("div", {
                                                    .class([
                                                        "text-gray-900",
                                                        "dark:text-gray-50",
                                                    ])
                                                    .text(&x.name)
                                                }),
                                                html!("div", {
                                                    .class([
                                                        "text-gray-800",
                                                        "dark:text-gray-200",
                                                        "text-sm"
                                                    ])
                                                    .text(&x.version)
                                                })
                                            ])
                                        })
                                    ])
                                }),
                                if x.has_update {
                                    html!("button", {
                                        .text("Update")
                                        .event(clone!(settings => move |_: events::Click| {
                                            Self::update_source(settings.clone(), x.id);
                                        }))
                                    })
                                } else {
                                    html!("div", {})
                                }
                            ])
                        })
                    )))
                }),
                html!("h1", {
                    .class([
                        "text-gray-900",
                        "dark:text-gray-100",
                    ])
                    .text("Available")
                }),
                html!("div", {
                    .class([
                        "rounded",
                        "bg-white",
                        "dark:bg-gray-900",
                        "shadow",
                        "dark:shadow-none",
                        "divide-y",
                        "divide-gray-200",
                        "dark:divide-gray-800",
                        "px-2"
                    ])
                    .children_signal_vec(settings.available_sources.signal_vec_cloned().map(clone!(settings => move |x|
                        html!("div", {
                            .class([
                                "p-2",
                                "flex",
                                "justify-between"
                            ])
                            .children(&mut [
                                html!("div", {
                                    .class("flex")
                                    .children(&mut [
                                        html!("img", {
                                            .class([
                                                "w-10",
                                                "h-10",
                                                "mr-2"
                                            ])
                                            .attribute("src", &["data:image/png;base64,", &x.icon].join(" "))
                                        }),
                                        html!("div", {
                                            .children(&mut [
                                                html!("div", {
                                                    .class([
                                                        "text-gray-900",
                                                        "dark:text-gray-50",
                                                    ])
                                                    .text(&x.name)
                                                }),
                                                html!("div", {
                                                    .class([
                                                        "text-gray-800",
                                                        "dark:text-gray-200",
                                                        "text-sm"
                                                    ])
                                                    .text(&x.version)
                                                })
                                            ])
                                        }),
                                    ])
                                }),
                                html!("button", {
                                    .text("Install")
                                    .event(clone!(settings => move |_: events::Click| {
                                        Self::install_source(settings.clone(), x.id);
                                    }))
                                })
                            ])
                        })
                    )))
                })
            ])
        })
    }

    pub fn render_source_settings(settings: Rc<Self>, source_id: i64) -> Dom {
        if source_id == 0 {
            Self::render_source_list(settings.clone())
        } else {
            let source = {
                let sources = settings.installed_sources.lock_ref();
                sources
                    .iter()
                    .find(|s| (*s).id == source_id)
                    .map(|s| s.clone())
                    .unwrap()
            };

            html!("div", {
                .class([
                    "flex",
                    "flex-col",
                    "items-center"
                ])
                .children(&mut [
                    html!("div", {
                        .class([
                            "flex",
                            "flex-col",
                            "items-center"
                        ])
                        .children(&mut [
                            html!("img", {
                                .class([
                                    "w-12",
                                    "h-12"
                                ])
                                .attribute("src", &["data:image/png;base64,", &source.icon].join(" "))
                            }),
                            html!("div", {
                                .text(&source.name)
                            }),
                            html!("div", {
                                .text(&source.version)
                            })
                        ])
                    }),
                    html!("button", {
                        .class([
                            "rounded",
                            "bg-gray-50",
                            "dark:bg-gray-900",
                            "text-red",
                            "w-full",
                            "p-2"
                        ])
                        .children(&mut [
                            html!("span", {
                                .class([
                                    "text-red-500",
                                    "mx-auto"
                                ])
                                .text("Uninstall")
                                .event(clone!(settings => move |_: events::Click| {
                                    Self::uninstall_source(settings.clone(), source_id);
                                }))
                            })
                        ])
                    })
                ])
            })
        }
    }

    pub fn render_users_management(settings: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "rounded",
                "bg-white",
                "dark:bg-gray-900",
                "shadow",
                "dark:shadow-none",
                "divide-y",
                "divide-gray-200",
                "dark:divide-gray-800",
                "px-2"
            ])
            .visible_signal(settings.me.signal_cloned().map(|me| {
                if let Some(me) = me {
                    if me.is_admin {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }))
            .children_signal_vec(settings.users.signal_vec_cloned().map(|x|
                html!("div", {
                    .class([
                        "p-2",
                        "flex",
                        "justify-between"
                    ])
                    .children(&mut [
                        html!("span", {
                            .text(&x.username)
                        }),
                        html!("span", {
                            .text(format!("{}", if x.is_admin { "Admin" } else { "" }).as_str())
                        })
                    ])
                })
            ))
        })
    }

    pub fn render_user(settings: Rc<Self>) -> Dom {
        link!(Route::Settings(SettingCategory::User).url(), {
            .class([
                "rounded",
                "bg-white",
                "dark:bg-gray-900",
                "shadow",
                "dark:shadow-none",
                "p-2",
                "flex",
                "mb-2",
                "items-center",
                "justify-between"
            ])
            .children(&mut [
                svg!("svg", {
                    .attribute("xmlns", "http://www.w3.org/2000/svg")
                    .attribute("viewBox", "0 0 24 24")
                    .attribute("stroke", "currentColor")
                    .attribute("fill", "none")
                    .class([
                        "w-12",
                        "h-12", 
                        "rounded-full",
                        "bg-gray-100",
                        "p-2",
                        "mr-2"
                    ])
                    .children(&mut [
                        svg!("path", {
                            .attribute("stroke-linecap", "round")
                            .attribute("stroke-linejoin", "round")
                            .attribute("stroke-width", "1")
                            .class("heroicon-ui")
                            .attribute("d", "M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z")
                        })
                    ])
                }),
                html!("div", {
                    .class([
                        "w-full"
                    ])
                    .child_signal(settings.me.signal_cloned().map(|me| {
                        if let Some(me) = me {
                            Some(html!("div", {
                                .class([
                                    "flex",
                                    "flex-col"
                                ])
                                .children(&mut [
                                    html!("span",{
                                        .text(&me.username)
                                    }),
                                    html!("span", {
                                        .class("text-sm")
                                        .text(format!("{}", if me.is_admin { "Admin" } else { "" }).as_str())
                                    })
                                ])
                            }))
                        } else {
                            None
                        }
                    }))
                }),
                svg!("svg", {
                    .attribute("xmlns", "http://www.w3.org/2000/svg")
                    .attribute("fill", "none")
                    .attribute("viewBox", "0 0 24 24")
                    .attribute("stroke", "currentColor")
                    .class("w-6")
                    .class("h-6")
                    .children(&mut [
                        svg!("path", {
                            .attribute("stroke-linecap", "round")
                            .attribute("stroke-linejoin", "round")
                            .attribute("stroke-width", "2")
                            .attribute("d", "M9 5l7 7-7 7")
                        })
                    ])
                })
            ])
        })
    }

    pub fn render(settings: Rc<Self>, category: SettingCategory) -> Dom {
        Self::fetch_me(settings.clone());
        settings.page.set(category.clone());
        match category {
            SettingCategory::Source(_) => Self::fetch_sources(settings.clone()),
            SettingCategory::Users => Self::fetch_user_list(settings.clone()),
            _ => {}
        }
        html!("div", {
            .class([
                "main",
                "w-full",
                "xl:flex"
            ])
            .children(&mut [
                Self::render_topbar(settings.clone()),
                html!("div", {
                    .class([
                        "w-full",
                        "p-2",
                        "ml-0",
                        "xl:ml-48"
                    ])
                    .child_signal(settings.page.signal_cloned().map(clone!(settings => move |x|
                        match x {
                            SettingCategory::None => Some(html!("div", {
                                .children(&mut [
                                    Self::render_user(settings.clone()),
                                    Self::render_categories(settings.clone())
                                ])
                            })),
                            SettingCategory::Reader => Some(ReaderSettings::render(settings.reader_settings.clone())),
                            SettingCategory::Source(source_id) => Some(Self::render_source_settings(settings.clone(), source_id)),
                            SettingCategory::Users => Some(Self::render_users_management(settings.clone())),
                            SettingCategory::User => Some(Profile::render(Profile::new())),
                            SettingCategory::CreateUser => Some(Login::render(Login::new())),
                        }
                    )))
                })
            ])
        })
    }
}
