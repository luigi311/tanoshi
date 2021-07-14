use std::rc::Rc;

use dominator::{clone, html, Dom};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use wasm_bindgen::prelude::*;

use crate::{common::{Cover, Spinner, css, events, snackbar}, query, utils::AsyncLoader};

pub struct Library {
    loader: AsyncLoader,
    spinner: Rc<Spinner>,
    cover_list: MutableVec<Cover>,
}

impl Library {
    pub fn new() -> Rc<Self> {
        Rc::new(Library {
            loader: AsyncLoader::new(),
            spinner: Spinner::new_with_fullscreen(true),
            cover_list: MutableVec::new(),
        })
    }

    pub fn fetch_libraries(library: Rc<Self>, refresh: bool) {
        library.spinner.set_active(true);
        library.loader.load(clone!(library => async move {
            match query::fetch_manga_from_favorite(refresh).await {
                Ok(covers) => {
                    let mut cover_list = library.cover_list.lock_mut();
                    cover_list.replace_cloned(covers);
                }
                Err(e) => {
                    snackbar::show(format!("failed to fetch library {}", e));
                }
            }
            library.spinner.set_active(false);
        }));
    }

    pub fn render_topbar(library: Rc<Self>) -> Dom {
        html!("div", {
            .class(css::TOPBAR_CLASS)
            .children(&mut [
                html!("button", {
                    .class("focus:outline-none")
                    .text("Filter")
                }),
                html!("span", {
                    .class([
                        "text-gray-50",
                        "focus:outline-none"
                    ])
                    .text("Library")
                }),
                html!("button", {
                    .class("focus:outline-none")
                    .text("Refresh")
                    .event(clone!(library => move |_: events::Click| {
                        Self::fetch_libraries(library.clone(), true);
                    }))
                })
            ])
        })
    }

    pub fn render_main(library: &Self) -> Dom {
        html!("div", {
            .class(["w-full",
                    "grid",
                    "grid-cols-3",
                    "md:grid-cols-4",
                    "lg:grid-cols-6",
                    "xl:grid-cols-12",
                    "gap-2",
                    "px-2",
                    "xl:pr-2",
                    "xl:pl-52",
                    "pb-safe-bottom-scroll"
            ])
            .children_signal_vec(library.cover_list.signal_vec_cloned().map(move |cover| cover.render()))
        })
    }

    pub fn render(library: Rc<Self>) -> Dom {
        Self::fetch_libraries(library.clone(), false);

        html!("div", {
            .class([
                "main",
            ])
            .children(&mut [
                Self::render_topbar(library.clone()),
                Self::render_main(&library),
                Spinner::render(&library.spinner)
            ])
        })
    }
}
