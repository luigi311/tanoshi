use std::rc::Rc;

use dominator::{clone, html, Dom};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use wasm_bindgen::prelude::*;

use crate::query::fetch_manga_from_favorite;
use crate::utils::AsyncLoader;
use crate::{
    app::App,
    common::{events, Cover, Spinner},
};

pub struct Library {
    loader: AsyncLoader,
    spinner: Rc<Spinner>,
    cover_list: MutableVec<Cover>,
}

impl Library {
    pub fn new() -> Rc<Self> {
        Rc::new(Library {
            loader: AsyncLoader::new(),
            spinner: Spinner::new(),
            cover_list: MutableVec::new(),
        })
    }

    pub fn render_topbar(spinner: Rc<Spinner>) -> Dom {
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
                    .text("Filter")
                }),
                html!("span", {
                    .class("text-gray-300")
                    .text("Library")
                }),
                html!("button", {
                    .text("Refresh")
                    .event(clone!(spinner => move |_: events::Click| {
                        // spinner.set_active(true);
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
            .children_signal_vec(library.cover_list.signal_vec_cloned().map(clone!(library => move |cover| cover.render())))
        })
    }

    pub fn render(library: Rc<Self>) -> Dom {
        if library.cover_list.lock_ref().len() == 0 {
            library.spinner.set_active(true);
            library.loader.load(clone!(library => async move {
                let covers = fetch_manga_from_favorite().await.unwrap_throw();
                let mut cover_list = library.cover_list.lock_mut();
                cover_list.replace_cloned(covers);
                library.spinner.set_active(false);
            }));
        }
        html!("div", {
            .class([
                "main",
            ])
            .children(&mut [
                Self::render_topbar(library.spinner.clone()),
                Self::render_main(&library),
                Spinner::render(&library.spinner)
            ])
        })
    }
}
