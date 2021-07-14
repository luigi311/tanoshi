use crate::common::css;
use crate::query;
use crate::utils::{AsyncLoader, local_storage, proxied_image_url};
use crate::{
    app::App,
    common::{Route, Spinner},
};
use chrono::NaiveDateTime;
use dominator::{clone, events, html, link, routing, svg, Dom};
use futures_signals::signal::{Signal, SignalExt};
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
    pub last_page_read: Mutable<Option<i64>>,
}

pub struct Manga {
    pub id: Mutable<i64>,
    pub source_id: Mutable<i64>,
    pub path: Mutable<String>,
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
    pub fn new(id: i64, source_id: i64, path: String) -> Rc<Self> {
        Rc::new(Self {
            id: Mutable::new(id),
            source_id: Mutable::new(source_id),
            path: Mutable::new(path),
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

    pub fn fetch_detail(manga: Rc<Self>, refresh: bool) {
        manga.loader.load(clone!(manga => async move {
            match query::fetch_manga_detail(manga.id.get(), refresh).await {
                Ok(result) => {
                    manga.title.set_neq(Some(result.title));
                    manga.author.lock_mut().replace_cloned(result.author);
                    manga.genre.lock_mut().replace_cloned(result.genre);
                    manga.cover_url.set_neq(Some(result.cover_url));
                    manga.description.set_neq(result.description);
                    manga.status.set_neq(result.status);
                    manga.is_favorite.set_neq(result.is_favorite);
                    manga.chapters.lock_mut().replace_cloned(result.chapters.iter().map(|chapter| Chapter{
                        id: chapter.id,
                        title: chapter.title.clone(),
                        uploaded: NaiveDateTime::parse_from_str(&chapter.uploaded, "%Y-%m-%dT%H:%M:%S%.f").expect_throw("failed to parse uploaded date"),
                        read_at: Mutable::new(chapter.read_at.as_ref().map(|time| NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%S%.f").expect_throw("failed to parse read at date"))),
                        last_page_read: Mutable::new(chapter.last_page_read)
                    }).collect());
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    pub fn fetch_detail_by_source_path(manga: Rc<Self>) {
        manga.loader.load(clone!(manga => async move {
            match query::fetch_manga_by_source_path(manga.source_id.get(), manga.path.get_cloned()).await {
                Ok(result) => {
                    manga.id.set_neq(result.id);
                    manga.title.set_neq(Some(result.title));
                    manga.author.lock_mut().replace_cloned(result.author);
                    manga.genre.lock_mut().replace_cloned(result.genre);
                    manga.cover_url.set_neq(Some(result.cover_url));
                    manga.description.set_neq(result.description);
                    manga.status.set_neq(result.status);
                    manga.is_favorite.set_neq(result.is_favorite);
                    manga.chapters.lock_mut().replace_cloned(result.chapters.iter().map(|chapter| Chapter{
                        id: chapter.id,
                        title: chapter.title.clone(),
                        uploaded: NaiveDateTime::parse_from_str(&chapter.uploaded, "%Y-%m-%dT%H:%M:%S%.f").expect_throw("failed to parse uploaded date"),
                        read_at: Mutable::new(chapter.read_at.as_ref().map(|time| NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%S%.f").expect_throw("failed to parse read at date"))),
                        last_page_read: Mutable::new(chapter.last_page_read)
                    }).collect());
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    pub fn add_to_or_remove_from_library(manga: Rc<Self>) {
        if manga.id.get() == 0 {
            return;
        }
        manga.loader.load(clone!(manga => async move {
            if manga.is_favorite.get() {
                match query::delete_from_library(manga.id.get()).await {
                    Ok(_) => {
                        manga.is_favorite.set_neq(false);
                    },
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            } else {
                match query::add_to_library(manga.id.get()).await {
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
            .class(css::TOPBAR_CLASS)
            .children(&mut [
                html!("button", {
                    .class([
                        "flex",
                        "items-center",
                        "focus:outline-none"
                    ])
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute("fill", "none")
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .class(["w-6", "h-6"])
                            .children(&mut [
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "2")
                                    .attribute("d", "M15 19l-7-7 7-7")
                                })
                            ])
                        }),
                    ])
                    .child_signal(manga.source_id.signal().map(|source_id|
                        match source_id {
                            0 => Some(html!("span", {
                                .text("Library")
                                .event(|_: events::Click| {
                                    routing::go_to_url("/");
                                })
                            })),
                            _ => Some(html!("span", {
                                .text("Catalogue")
                                .event(|_: events::Click| {
                                    let history = window().unwrap().history().unwrap();
                                    if history.length().unwrap() > 1 {
                                        let _ = history.back();
                                    } else {
                                        routing::go_to_url("/");
                                    }
                                })
                            })),
                        }
                    ))
                }),
                html!("span", {
                    .class(["text-gray-50", "truncate", "mx-auto", "px-2"])
                    .text_signal(manga.title.signal_cloned().map(|x| x.unwrap_or("".to_string())))
                }),
                html!("button", {
                    .class("focus:outline-none")
                    .text("Refresh")
                    .event(clone!(manga => move |_: events::Click| {
                        if manga.id.get() != 0 {
                            manga.title.set_neq(None);
                            manga.author.lock_mut().clear();
                            manga.genre.lock_mut().clear();
                            manga.cover_url.set_neq(None);
                            manga.description.set_neq(None);
                            manga.status.set_neq(None);
                            manga.is_favorite.set_neq(false);
                            manga.chapters.lock_mut().clear();

                            Self::fetch_detail(manga.clone(), true);
                        }
                    }))
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
                                        .attribute("src", &proxied_image_url(&cover_url))
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
                    .class([
                        "rounded", 
                        "p-2", 
                        "w-10", 
                        "h-10", 
                        "bg-white", 
                        "dark:bg-gray-900", 
                        "shadow", 
                        "dark:shadow-none", 
                        "text-gray-900", 
                        "dark:text-gray-100",
                        "focus:outline-none"
                    ])
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
                        link!(Route::Chapter(chapter.id, 0).url(), {
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
                                                html!("div", {
                                                    .children(&mut [
                                                        html!("span", {
                                                            .class("text-sm")
                                                            .text(&chapter.uploaded.date().to_string())
                                                        }),
                                                    ])
                                                    .child_signal(chapter.last_page_read.signal().map(|x| match x {
                                                        None => None,
                                                        Some(page) => Some(html!("span", {
                                                            .class(["mx-2", "text-sm"])
                                                            .text(format!("Page: {}", page + 1).as_str())
                                                        }))
                                                    }))
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
        if manga_page.id.get() != 0 {
            Self::fetch_detail(manga_page.clone(), false);
        } else if manga_page.source_id.get() != 0 && manga_page.path.get_cloned() != "" {
            Self::fetch_detail_by_source_path(manga_page.clone());
        }

        html!("div", {
            .class(["main", "w-full", "2xl:w-1/2", "mx-auto", "px-2", "flex", "flex-col"])
            .children(&mut [
                Self::render_topbar(manga_page.clone()),
                html!("div", {
                    .class(["animate__animated", "animate__fadeIn"])
                    .children(&mut [
                        Self::render_header(manga_page.clone()),
                        Self::render_description(manga_page.clone()),
                        Self::render_chapters(manga_page.clone())
                    ])
                })
            ])
        })
    }
}
