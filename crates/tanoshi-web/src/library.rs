use std::rc::Rc;

use dominator::{clone, html, with_node, Dom};
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::{MutableVec, SignalVecExt},
};
use web_sys::HtmlInputElement;

use crate::{
    common::{events, snackbar, Cover, Spinner},
    query,
    utils::AsyncLoader,
};

pub struct Library {
    keyword: Mutable<String>,
    is_search: Mutable<bool>,
    loader: AsyncLoader,
    spinner: Rc<Spinner>,
    cover_list: MutableVec<Cover>,
}

impl Library {
    pub fn new() -> Rc<Self> {
        Rc::new(Library {
            keyword: Mutable::new("".to_string()),
            is_search: Mutable::new(false),
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
            .class("topbar")
            .child_signal(library.is_search.signal().map(clone!(library => move |is_search|
                (!is_search).then(|| html!("button", {
                    .text("Refresh")
                    .event(clone!(library => move |_: events::Click| {
                        Self::fetch_libraries(library.clone(), true);
                    }))
                }))
            )))
            .child_signal(library.is_search.signal().map(clone!(library => move |is_search| {
                if is_search {
                    Some(html!("input" => HtmlInputElement, {
                        .style("width", "100%")
                        .attribute("placeholder", "Search")
                        .attribute("type", "text")
                        .property_signal("value", library.keyword.signal_cloned())
                        .with_node!(input => {
                            .event(clone!(library => move |_: events::Input| {
                                library.keyword.set_neq(input.value());
                            }))
                            .event_preventable(|event: events::KeyDown| {
                                if event.key() == "Enter" {
                                    event.prevent_default();
                                }
                            })
                        })
                    }))
                } else {
                    Some(html!("span", {
                        .text("Library")
                    }))
                }
            })))
            .child_signal(library.is_search.signal().map(clone!(library => move |is_search| {
                if is_search {
                    Some(html!("button", {
                        .text("Cancel")
                        .event(clone!(library => move |_: events::Click| {
                            library.is_search.set_neq(false);
                            if library.keyword.get_cloned() != "" {
                                library.keyword.set_neq("".to_string());
                            }
                        }))
                    }))
                } else {
                    Some(html!("button", {
                        .text("Search")
                        .event(clone!(library => move |_: events::Click| {
                            library.is_search.set_neq(true);
                        }))
                    }))
                }
            })))
        })
    }

    pub fn render_main(library: Rc<Self>, keyword: String) -> Dom {
        html!("div", {
            .class("manga-grid")
            .children_signal_vec(library.cover_list.signal_vec_cloned().filter_map(clone!(keyword => move |cover| {
                if keyword.is_empty() || cover.title.to_ascii_lowercase().contains(&keyword.to_ascii_lowercase()) {
                    Some(cover.render())
                } else {
                    None
                }
            })))
        })
    }

    pub fn render(library: Rc<Self>) -> Dom {
        Self::fetch_libraries(library.clone(), false);

        html!("div", {
            .class("page")
            .children(&mut [
                Self::render_topbar(library.clone()),
                html!("div", {
                    .class("topbar-spacing")
                }),
                Spinner::render(&library.spinner)
            ])
            .child_signal(
                library.keyword.signal_cloned().map(
                    clone!(library => move |keyword| Some(Self::render_main(library.clone(), keyword)))
                )
            )
        })
    }
}
