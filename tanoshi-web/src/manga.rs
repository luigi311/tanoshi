use crate::query::{add_to_library, delete_from_library, fetch_manga_detail};
use crate::utils::{proxied_image_url, AsyncLoader};
use crate::{
    app::App,
    common::{Route, Spinner},
};
use chrono::NaiveDateTime;
use dominator::{clone, events, html, link, routing, svg, Dom};
use futures_signals::signal::SignalExt;
use futures_signals::{
    signal::Mutable,
    signal_vec::{MutableVec, SignalVecExt},
};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::window;

#[derive(Clone)]
struct Chapter {
    pub id: i64,
    pub title: String,
    pub uploaded: NaiveDateTime,
    pub read_at: Mutable<Option<NaiveDateTime>>,
}

pub struct Manga {
    pub id: i64,
    title: Mutable<Option<String>>,
    author: MutableVec<String>,
    genre: MutableVec<String>,
    cover_url: Mutable<Option<String>>,
    description: Mutable<Option<String>>,
    status: Mutable<Option<String>>,
    is_favorite: Mutable<bool>,
    chapters: MutableVec<Chapter>,
    loader: AsyncLoader,
}

impl Manga {
    pub fn new(id: i64) -> Rc<Self> {
        Rc::new(Self {
            id,
            title: Mutable::new(None),
            author: MutableVec::new(),
            genre: MutableVec::new(),
            cover_url: Mutable::new(None),
            description: Mutable::new(None),
            status: Mutable::new(None),
            is_favorite: Mutable::new(false),
            chapters: MutableVec::new(),
            loader: AsyncLoader::new(),
        })
    }

    pub fn fetch_detail(manga: Rc<Self>) {
        manga.loader.load(clone!(manga => async move {
            match fetch_manga_detail(manga.id).await {
                Ok(result) => {
                    manga.title.lock_mut().replace(result.title);
                    manga.author.lock_mut().replace_cloned(result.author);
                    manga.genre.lock_mut().replace_cloned(result.genre);
                    manga.cover_url.lock_mut().replace(result.cover_url);
                    manga.description.lock_mut().replace(result.description.unwrap_throw());
                    manga.status.lock_mut().replace(result.status.unwrap_throw());
                    manga.is_favorite.set_neq(result.is_favorite);
                    manga.chapters.lock_mut().replace_cloned(result.chapters.iter().map(|chapter| Chapter{
                        id: chapter.id,
                        title: chapter.title.clone(),
                        uploaded: NaiveDateTime::parse_from_str(&chapter.uploaded, "%Y-%m-%d %H:%M:%S").unwrap_throw(),
                        read_at: Mutable::new(chapter.read_at.as_ref().map(|time| NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S").unwrap_throw())),
                    }).collect());
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    pub fn add_to_or_remove_from_library(manga: Rc<Self>) {
        manga.loader.load(clone!(manga => async move {
            if manga.is_favorite.get() {
                match delete_from_library(manga.id).await {
                    Ok(_) => {
                        manga.is_favorite.set_neq(false);
                    },
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            } else {
                match add_to_library(manga.id).await {
                    Ok(_) => {
                        manga.is_favorite.set_neq(true);
                    },
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            }
        }));
    }

    pub fn render_topbar(manga: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "w-full",
                "pl-2",
                "pr-2",
                "pb-2",
                "flex",
                "justify-between",
                "fixed",
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
                    .class(["text-gray-300", "truncate", "mx-auto", "px-2"])
                    .text_signal(manga.title.signal_cloned().map(|x| x.unwrap_or("".to_string())))
                }),
                html!("button", {
                    .text("Refresh")
                    .event(|_: events::Click| {
                    })
                }),
            ])
        })
    }

    pub fn render_header(manga: Rc<Self>) -> Dom {
        html!("div", {
            .attribute("id", "detail")
            .class([
                "flex",
                "flex-col",
                "justify-center",
                "p-2",
                "mb-2",
                "rounded"
            ])
            .children(&mut [
                html!("div", {
                    .class("flex")
                    .class_signal("animate-pulse", manga.loader.is_loading())
                    .children(&mut [
                        html!("div", {
                            .class(["pb-7/6", "mr-2"])
                            .child_signal(manga.cover_url.signal_cloned().map(|x| {
                                if let Some(cover_url) = x {
                                    Some(html!("img", {
                                        .class(["w-32", "rounded", "object-cover"])
                                        .attribute("src", &cover_url)
                                        .attribute("loading", "lazy")
                                    }))
                                } else {
                                    Some(html!("div", {
                                        .class(["w-32", "h-44", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                                    }))
                                }
                            }))
                        }),
                        html!("div", {
                            .class(["flex", "flex-col"])
                            .children(&mut [
                                html!("div", {
                                    .child_signal(manga.title.signal_cloned().map(|x| {
                                        if let Some(title) = x {
                                            Some(html!("span", {
                                                .class(["md:text-xl", "sm:text-md", "text-gray-900", "dark:text-gray-300", "mr-2"])
                                                .text(title.as_str())
                                            }))
                                        } else {
                                            Some(html!("div", {
                                                .class(["w-32", "md:w-48", "lg:w-64", "xl:w-72", "h-6", "mb-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                                            }))
                                        }
                                    }))
                                }),
                                html!("div", {
                                    .child_signal(manga.loader.is_loading().map(|x| {
                                        if x {
                                            Some(html!("div", {
                                                .class(["w-32", "md:w-48", "lg:w-64", "xl:w-72", "h-6", "mb-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                                            }))
                                        } else {
                                            None
                                        }
                                    }))
                                    .children_signal_vec(manga.author.signal_vec_cloned().map(|x| {
                                        html!("span", {
                                            .class(["md:text-lg", "sm:text-sm", "text-gray-900", "dark:text-gray-300", "mr-2"])
                                            .text(&x)
                                        })
                                    }))
                                }),
                                html!("div", {
                                    .child_signal(manga.status.signal_cloned().map(|x|
                                        if let Some(status) = x {
                                            Some(html!("span", {
                                                .class(["md:text-lg", "sm:text-sm", "text-gray-900", "dark:text-gray-300", "mr-2"])
                                                .text(&status)
                                            }))
                                        } else {
                                            Some(html!("div", {
                                                .class(["w-32", "md:w-48", "lg:w-64", "xl:w-72", "h-6", "mb-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                                            }))
                                        }
                                    ))
                                })
                            ])
                        })
                    ])
                })
            ])
        })
    }

    pub fn render_description(manga: Rc<Self>) -> Dom {
        html!("div", {
            .attribute("id", "description")
            .class([
                "flex",
                "flex-col",
                "justify-center",
                "p-2",
                "mb-2",
                "rounded"
            ])
            .class_signal("animate-pulse", manga.loader.is_loading())
            .children(&mut [
                html!("button", {
                    .class(["rounded", "p-2", "w-10", "h-10", "bg-white", "dark:bg-gray-900", "shadow", "dark:shadow-none", "text-gray-900", "dark:text-gray-100"])
                    .visible_signal(manga.loader.is_loading().map(|x| !x))
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute_signal("fill", manga.is_favorite.signal().map(|x| if x { "currentColor" } else { "none" }))
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .children(&mut [
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "1")
                                    .attribute("d", "M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z")
                                })
                            ])
                        })
                    ])
                    .event(clone!(manga => move |_: events::Click| {
                        Self::add_to_or_remove_from_library(manga.clone());
                    }))
                }),
                html!("span", {
                    .class(["md:text-xl", "sm:text-base", "font-bold", "text-gray-900", "dark:text-gray-300"])
                    .text("Description")
                })
            ])
            .child_signal(manga.description.signal_cloned().map(|x| {
                if let Some(description) = x {
                    Some(html!("p", {
                        .class(["break-normal", "md:text-base", "sm:text-xs", "text-gray-900", "dark:text-gray-300"])
                        .text(&description)
                    }))
                } else {
                    Some(html!("div", {
                        .class(["flex", "flex-col"])
                        .children(&mut [
                            html!("div", {
                                .class(["w-full", "h-6", "mb-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                            }),
                            html!("div", {
                                .class(["w-full", "h-6", "mb-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                            }),
                            html!("div", {
                                .class(["w-full", "h-6", "mb-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                            })
                        ])
                    }))
                }
            }))
            .children(&mut [
                html!("div", {
                    .class(["w-full", "flex", "flex-wrap"])
                    .children_signal_vec(manga.genre.signal_vec_cloned().map(|x| {
                        html!("span", {
                            .class(["md:text-base", "sm:text-xs", "text-gray-900", "dark:text-gray-300", "mr-2", "rounded-full", "border", "border-gray-900", "dark:border-gray-300", "px-2", "mt-2"])
                            .text(&x)
                        })
                    }))
                })
            ])
        })
    }

    pub fn render_chapters(manga: Rc<Self>) -> Dom {
        html!("div", {
            .attribute("id", "description")
            .class([
                "flex",
                "justify-center",
                "p-2",
                "rounded"
            ])
            .children(&mut [
                html!("div", {
                    .class(["flex", "flex-col", "w-full", "divide-y", "dark:divide-gray-800", "divide-gray-300"])
                    .class_signal("animate-pulse", manga.loader.is_loading())
                    .children(&mut [
                        html!("span", {
                            .class(["md:text-xl", "sm:text-base", "font-bold", "text-gray-900", "dark:text-gray-300"])
                            .text("Chapters")
                        }),
                    ])
                    .child_signal(manga.loader.is_loading().map(|x| {
                        if x {
                            Some(html!("div", {
                                .class(["flex", "flex-col"])
                                .children(&mut [
                                    html!("div", {
                                        .class(["w-full", "h-6", "my-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                                    }),
                                    html!("div", {
                                        .class(["w-full", "h-6", "mb-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                                    }),
                                    html!("div", {
                                        .class(["w-full", "h-6", "mb-2", "rounded", "bg-gray-200", "dark:bg-gray-800"])
                                    })
                                ])
                            }))
                        } else {
                            None
                        }
                    }))
                    .children_signal_vec(manga.chapters.signal_vec_cloned().map(|chapter| {
                        link!(Route::Chapter(chapter.id).url(), {
                            .class(["flex", "inline-flex", "hover:bg-gray-200", "dark:hover:bg-gray-700", "p-2", "text-gray-900", "dark:text-gray-300"])
                            .class_signal("text-opacity-50", chapter.read_at.signal().map(|x| x.is_some()))
                            .children(&mut [
                                html!("div", {
                                    .class(["flex", "justify-between", "items-center", "w-full"])
                                    .children(&mut [
                                        html!("div", {
                                            .class(["flex", "flex-col"])
                                            .children(&mut [
                                                html!("span", {
                                                    .class(["text-md", "font-semibold"])
                                                    .text(&chapter.title)
                                                }),
                                                html!("span", {
                                                    .class("text-sm")
                                                    .text(&chapter.uploaded.date().to_string())
                                                })
                                            ])
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
                            ])
                        })
                    }))
                })
            ])
        })
    }

    pub fn render(manga_page: Rc<Self>) -> Dom {
        Self::fetch_detail(manga_page.clone());
        html!("div", {
            .class(["main", "w-full", "2xl:w-1/2", "mx-auto", "px-2", "flex", "flex-col"])
            .children(&mut [
                Self::render_topbar(manga_page.clone()),
                Self::render_header(manga_page.clone()),
                Self::render_description(manga_page.clone()),
                Self::render_chapters(manga_page.clone())
            ])
        })
    }
}
