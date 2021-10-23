use std::rc::Rc;

use crate::utils::history;
use crate::{
    common::snackbar,
    query::{
        self,
        browse_source::{SortByParam, SortOrderParam},
    },
    utils::local_storage,
};
use crate::{
    common::{Cover, Spinner},
    utils::AsyncLoader,
};
use dominator::{clone, events, html, svg, with_node, Dom};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::HtmlInputElement;

pub const STORAGE_KEY: &str = "catalogue";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct Source {
    id: i64,
    name: String,
    version: String,
    icon: String,
    need_login: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct SourceManga {
    name: String,
    covers: Vec<Cover>,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Catalogue {
    source_id: i64,
    keyword: Mutable<String>,
    page: Mutable<i64>,
    sort_by: Mutable<SortByParam>,
    sort_order: Mutable<SortOrderParam>,
    is_search: Mutable<bool>,
    cover_list: MutableVec<Cover>,
    #[serde(skip)]
    loader: AsyncLoader,
    #[serde(skip)]
    spinner: Rc<Spinner>,
}

impl Default for Catalogue {
    fn default() -> Self {
        Self {
            source_id: 0,
            keyword: Mutable::new("".to_string()),
            page: Mutable::new(1),
            sort_by: Mutable::new(SortByParam::VIEWS),
            sort_order: Mutable::new(SortOrderParam::DESC),
            is_search: Mutable::new(false),
            cover_list: MutableVec::new(),
            spinner: Spinner::new(),
            loader: AsyncLoader::new(),
        }
    }
}

impl Catalogue {
    pub fn new(source_id: i64) -> Rc<Self> {
        let catalogue = local_storage()
            .get(STORAGE_KEY)
            .unwrap_throw()
            .and_then(|object_str| serde_json::from_str(&object_str).ok())
            .unwrap_or_else(|| Self {
                source_id,
                ..Default::default()
            });

        Rc::new(catalogue)
    }

    pub fn serialize_into_json(&self) -> String {
        serde_json::to_string(self).unwrap_throw()
    }

    pub fn fetch_mangas(catalogue: Rc<Self>) {
        catalogue.replace_state_with_url();
        catalogue.spinner.set_active(true);
        catalogue.loader.load(clone!(catalogue => async move {
            match query::fetch_manga_from_source(catalogue.source_id, catalogue.page.get(), Some(catalogue.keyword.get_cloned()), catalogue.sort_by.get_cloned(), catalogue.sort_order.get_cloned()).await {
                Ok(covers) => {
                    let mut cover_list = catalogue.cover_list.lock_mut();
                    if catalogue.page.get() == 1 {
                        cover_list.replace_cloned(covers);
                    } else if catalogue.page.get() > 0 {
                        cover_list.extend(covers);
                    }

                }
                Err(e) => {
                    snackbar::show(format!("Fetch manga from source failed: {}", e))
                }
            }

            let state =  catalogue.serialize_into_json();
            local_storage().set(STORAGE_KEY, state.as_str()).unwrap_throw();
            catalogue.spinner.set_active(false);
        }));
    }

    fn replace_state_with_url(&self) {
        let sort_by = serde_plain::to_string(&self.sort_by.lock_ref().clone()).unwrap();
        let sort_order = serde_plain::to_string(&self.sort_order.lock_ref().clone()).unwrap();
        let url = if !self.keyword.lock_ref().is_empty() {
            format!(
                "/catalogue/{}?keyword={}&sort_by={}&sort_order={}",
                self.source_id,
                self.keyword.lock_ref().clone(),
                sort_by,
                sort_order
            )
        } else {
            format!(
                "/catalogue/{}?sort_by={}&sort_order={}",
                self.source_id, sort_by, sort_order
            )
        };

        if let Err(e) = history().replace_state_with_url(&JsValue::null(), "", Some(&url)) {
            let message = if let Some(msg) = e.as_string() {
                msg
            } else {
                "unknown reason".to_string()
            };

            error!("error replace_state_with_url: {}", message);
        }
    }

    pub fn render_topbar(catalogue: Rc<Self>) -> Dom {
        html!("div", {
            .class("topbar")
            .child_signal(catalogue.is_search.signal().map(|is_search| {
                if is_search {
                    None
                } else {
                    Some(html!("button", {
                        .attribute("id", "filter")
                        .children(&mut [
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
                                        .attribute("d", "M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z")
                                    })
                                ])
                            }),
                        ])
                    }))
                }
            }))
            .child_signal(catalogue.is_search.signal().map(clone!(catalogue => move |is_search| {
                if is_search {
                    Some(html!("input" => HtmlInputElement, {
                        .style("width", "100%")
                        .attribute("placeholder", "Search")
                        .attribute("type", "text")
                        .attribute("value", &catalogue.keyword.get_cloned())
                        .with_node!(input => {
                            .event_preventable(clone!(catalogue => move |e: events::KeyDown| {
                                if e.key() == "Enter" {
                                    e.prevent_default();
                                    catalogue.cover_list.lock_mut().clear();
                                    catalogue.page.set_neq(1);
                                    catalogue.keyword.set_neq(input.value());
                                }
                            }))
                        })
                    }))
                } else {
                    Some(html!("span", {
                        .text("Catalogue")
                    }))
                }
            })))
            .child_signal(catalogue.is_search.signal().map(clone!(catalogue => move |is_search| {
                if is_search {
                    Some(html!("button", {
                        .attribute("id", "cancel")
                        .event(clone!(catalogue => move |_: events::Click| {
                            catalogue.is_search.set_neq(false);
                            if catalogue.keyword.get_cloned() != "" {
                                catalogue.keyword.set_neq("".to_string());
                                catalogue.cover_list.lock_mut().clear();
                                catalogue.page.set_neq(1);
                            }
                        }))
                        .children(&mut [
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
                                        .attribute("d", "M6 18L18 6M6 6l12 12")
                                    })
                                ])
                            }),
                        ])
                    }))
                } else {
                    Some(html!("button", {
                        .attribute("id", "search")
                        .event(clone!(catalogue => move |_: events::Click| {
                            catalogue.is_search.set_neq(true);
                        }))
                        .children(&mut [
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
                                        .attribute("d", "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z")
                                    })
                                ])
                            }),
                        ])
                    }))
                }
            })))
        })
    }

    pub fn render_main(catalogue: Rc<Self>) -> Dom {
        html!("div", {
            .style("padding", "0.5rem")
            .children(&mut [
                html!("div", {
                    .class("manga-grid")
                    .children_signal_vec(catalogue.cover_list.signal_vec_cloned().map(|cover| cover.render()))
                }),
                html!("div", {
                    .class("load-more-btn")
                    .child_signal(catalogue.spinner.signal().map(clone!(catalogue => move |x| if x {
                        Some(Spinner::render(&catalogue.spinner))
                    } else {
                        Some(html!("button", {
                            .text("Load More")
                            .event(clone!(catalogue => move |_: events::Click| {
                                catalogue.page.set(catalogue.page.get() + 1);
                                Self::fetch_mangas(catalogue.clone());
                            }))
                        }))
                    })))
                })
            ])
        })
    }

    pub fn render(
        self: Rc<Self>,
        keyword: Option<String>,
        sort_by: SortByParam,
        sort_order: SortOrderParam,
    ) -> Dom {
        let s = map_ref! {
            let keyword = self.keyword.signal_cloned(),
            let sort_by = self.sort_by.signal_cloned(),
            let sort_order = self.sort_order.signal_cloned() =>

            (keyword.clone(), sort_by.clone(), sort_order.clone())
        };

        if let Some(keyword) = keyword {
            self.keyword.set_neq(keyword);
            self.is_search.set_neq(true);
        }
        self.sort_by.set_neq(sort_by);
        self.sort_order.set_neq(sort_order);

        html!("div", {
            .future(s.for_each({
                let catalogue = self.clone();
                move |_| {
                    catalogue.replace_state_with_url();
                    Self::fetch_mangas(catalogue.clone());

                    async {}
                }
            }))
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
