use std::rc::Rc;

use crate::{common::Spinner, utils::AsyncLoader};
use crate::{
    common::{snackbar, Category, Route},
    query,
    utils::is_tauri_signal,
};
use dominator::{clone, html, link, routing, svg, Dom};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};

pub struct LibraryList {
    categories: MutableVec<Category>,
    loader: AsyncLoader,
    spinner: Rc<Spinner>,
}

impl LibraryList {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            categories: MutableVec::new(),
            loader: AsyncLoader::new(),
            spinner: Spinner::new(),
        })
    }

    pub fn fetch_categories(library: Rc<Self>) {
        library.spinner.set_active(true);
        library.loader.load(clone!(library => async move {
            match query::fetch_categories().await {
                Ok(res) => {
                    if res.len() == 0 {
                        routing::go_to_url(&Route::Library(None).url());
                        return
                    }

                    library.categories.lock_mut().replace_cloned(res.into_iter().map(|c| Category{
                        id: c.id,
                        name: c.name.clone(),
                    }).collect());
                }
                Err(e) => {
                    snackbar::show(format!("failed to fetch categories {}", e));
                }
            }
            library.spinner.set_active(false);
        }));
    }

    pub fn render_topbar(_library: Rc<Self>) -> Dom {
        html!("div", {
            .class("topbar")
            .class_signal("tauri", is_tauri_signal())
            .children(&mut [
                html!("span", {
                    .style("margin", "auto")
                    .text("Library")
                })
            ])
        })
    }

    pub fn render_main(library: Rc<Self>) -> Dom {
        html!("ul", {
                    .class("list")
                    .children(&mut [
                        html!("li", {
                            .class("list-item")
                            .children(&mut [
                                link!(Route::Library(None).url(), {
                                    .class("source-item")
                                    .children(&mut [
                                        html!("div", {
                                            .style("margin", "0.5rem")
                                            .style("width", "1.5rem")
                                            .style("height", "1.5rem")
                                        }),
                                        html!("span", {
                                            .text("Default")
                                        }),
                                    ])
                                }),
                            ])
                        })
                    ])
                    .children_signal_vec(library.categories.signal_vec_cloned().map(|cat| html!("li", {
                        .class("list-item")
                        .children(&mut [
                            link!(Route::Library(Some(cat.id)).url(), {
                                .class("source-item")
                                .children(&mut [
                                    /*
                                    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
        </svg>
                                    */
                                    svg!("svg", {
                                        .attribute("xmlns", "http://www.w3.org/2000/svg")
                                        .attribute("fill", "none")
                                        .attribute("viewBox", "0 0 24 24")
                                        .attribute("stroke", "currentColor")
                                        .class("icon")
                                        .children(&mut [
                                            svg!("path", {
                                                .attribute("stroke-linecap", "round")
                                                .attribute("stroke-linejoin", "round")
                                                .attribute("stroke-width", "2")
                                                .attribute("d", "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z")
                                            })
                                        ])
                                    }),
                                    html!("span", {
                                        .text(&cat.name)
                                    }),
                                ])
                            }),
                        ])
                    })))
                })
    }

    pub fn render(self: Rc<Self>) -> Dom {
        Self::fetch_categories(self.clone());
        html!("div", {
            .class("main")
            .children(&mut [
                Self::render_topbar(self.clone()),
                html!("div", {
                    .class("topbar-spacing")
                })
            ])
            .children(&mut [
                Self::render_main(self.clone()),
                html!("div", {
                    .class("bottombar-spacing")
                })
            ])
        })
    }
}
