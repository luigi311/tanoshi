use std::rc::Rc;

use dominator::{clone, events, html, link, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use wasm_bindgen::prelude::*;

use crate::common::{css, snackbar};
use crate::{
    common::Route,
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

#[derive(Debug, Clone)]
pub struct Source {
    id: i64,
    name: String,
    version: String,
    icon: String,
    need_login: bool,
}

pub struct Catalogue {
    pub source_id: Mutable<i64>,
    keyword: Mutable<String>,
    page: Mutable<i64>,
    sort_by: Mutable<SortByParam>,
    sort_order: Mutable<SortOrderParam>,
    is_search: Mutable<bool>,
    cover_list: MutableVec<Cover>,
    sources: MutableVec<Source>,
    loader: AsyncLoader,
    spinner: Rc<Spinner>,
}

impl Catalogue {
    pub fn new() -> Rc<Self> {
        let source_id = local_storage()
            .get("catalogue_id")
            .unwrap_throw()
            .unwrap_or("0".to_string())
            .parse::<i64>()
            .unwrap_or(0);
        let keyword = match local_storage().get("catalogue_keyword").unwrap_throw() {
            Some(value) => value.to_string(),
            None => "".to_string()
        };
        let page = match local_storage().get("catalogue_page").unwrap_throw() {
            Some(value) => value.parse::<i64>().unwrap_or(1),
            None => 1
        };
        let sort_by = match local_storage().get("catalogue_sort_by").unwrap_throw() {
            Some(value) => serde_json::from_str(&value).unwrap_or(SortByParam::VIEWS),
            None => SortByParam::VIEWS
        };
        let sort_order = match local_storage().get("catalogue_sort_order").unwrap_throw() {
            Some(value) => serde_json::from_str(&value).unwrap_or(SortOrderParam::DESC),
            None => SortOrderParam::DESC
        };
        let is_search = match local_storage().get("catalogue_is_search").unwrap_throw() {
            Some(value) => value.parse::<bool>().unwrap_or(false),
            None => false
        };
        let cover_list = match local_storage().get("catalogue_cover_list").unwrap_throw() {
            Some(value) => serde_json::from_str(&value).unwrap_or(MutableVec::new()),
            None => MutableVec::new()
        };

        Rc::new(Catalogue {
            source_id: Mutable::new(source_id),
            keyword: Mutable::new(keyword),
            page: Mutable::new(page),
            sort_by: Mutable::new(sort_by),
            sort_order: Mutable::new(sort_order),
            is_search: Mutable::new(is_search),
            cover_list,
            spinner: Spinner::new(),
            sources: MutableVec::new(),
            loader: AsyncLoader::new(),
        })
    }

    pub fn reset(&self) {
        local_storage().delete("catalogue_id").unwrap_throw();
        local_storage().delete("catalogue_keyword").unwrap_throw();
        local_storage().delete("catalogue_page").unwrap_throw();
        local_storage().delete("catalogue_sort_by").unwrap_throw();
        local_storage().delete("catalogue_sort_order").unwrap_throw();
        local_storage().delete("catalogue_is_search").unwrap_throw();
        local_storage().delete("catalogue_cover_list").unwrap_throw();

        self.source_id.set(0);
        self.keyword.set("".to_string());
        self.page.set(1);
        self.sort_by.set(SortByParam::VIEWS);
        self.sort_order.set(SortOrderParam::DESC);
        self.is_search.set(false);
        self.cover_list.lock_mut().clear();
        self.sources.lock_mut().clear();
    }

    pub fn fetch_mangas(catalogue: Rc<Self>) {
        catalogue.spinner.set_active(true);
        catalogue.loader.load(clone!(catalogue => async move {
            match query::fetch_manga_from_source(*catalogue.source_id.lock_ref(), catalogue.page.get(), Some(catalogue.keyword.get_cloned()), catalogue.sort_by.get_cloned(), catalogue.sort_order.get_cloned()).await {
                Ok(covers) => {
                    let mut cover_list = catalogue.cover_list.lock_mut();
                    if catalogue.page.get() == 1 {
                        cover_list.replace_cloned(covers);
                    } else if catalogue.page.get() > 0 {
                        cover_list.extend(covers);
                    }

                    local_storage().set("catalogue_keyword", &catalogue.keyword.lock_ref()).unwrap_throw();
                    local_storage().set("catalogue_page", &catalogue.page.lock_ref().to_string()).unwrap_throw();
                    local_storage().set("catalogue_sort_by", &serde_json::to_string(&*catalogue.sort_by.lock_ref()).unwrap_throw()).unwrap_throw();
                    local_storage().set("catalogue_sort_order", &serde_json::to_string(&*catalogue.sort_order.lock_ref()).unwrap_throw()).unwrap_throw();
                    local_storage().set("catalogue_is_search", &serde_json::to_string(&*catalogue.is_search.lock_ref()).unwrap_throw()).unwrap_throw();
                    local_storage().set("catalogue_cover_list", serde_json::to_string(&*cover_list).unwrap_or("".to_string()).as_str()).unwrap_throw();
                }
                Err(e) => {
                    snackbar::show(format!("Fetch manga from source failed: {}", e))
                }
            }
            
            catalogue.spinner.set_active(false);
        }));
    }

    pub fn fetch_sources(catalogue: Rc<Self>) {
        catalogue.loader.load(clone!(catalogue => async move {
            match query::fetch_sources().await {
                Ok(result) => {
                    catalogue.sources.lock_mut().replace_cloned(result.iter().map(|s| Source {
                        id: s.id,
                        name: s.name.clone(),
                        version: s.version.clone(),
                        icon: s.icon.clone(),
                        need_login: s.need_login,
                    }).collect()
                )},
                Err(err) => {
                    snackbar::show(format!("{}", err));
                }
            }
        }));
    }

    pub fn render_topbar(catalogue: Rc<Self>) -> Dom {
        html!("div", {
            .class("topbar")
            .child_signal(catalogue.is_search.signal().map(|is_search| {
                if is_search {
                    None
                } else {
                    Some(html!("button", {
                        .text("Filter")
                    }))
                }
            }))
            .child_signal(catalogue.is_search.signal().map(clone!(catalogue => move |is_search| {
                if is_search {
                    Some(html!("input", {
                        .attribute("placeholder", "Search")
                        .attribute("type", "text")
                        .property_signal("value", catalogue.keyword.signal_cloned())
                        .event(clone!(catalogue => move |event: events::Input| {
                            catalogue.keyword.set_neq(event.value().unwrap_throw());
                        }))
                        .event_preventable(clone!(catalogue => move |event: events::KeyDown| {
                            if event.key() == "Enter" {
                                event.prevent_default();
                                catalogue.cover_list.lock_mut().clear();
                                catalogue.page.set_neq(1);
                                Self::fetch_mangas(catalogue.clone());
                            }
                        }))
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
                        .text("Cancel")
                        .event(clone!(catalogue => move |_: events::Click| {
                            catalogue.is_search.set_neq(false);
                            if catalogue.keyword.get_cloned() != "" {
                                catalogue.keyword.set_neq("".to_string());
                                catalogue.cover_list.lock_mut().clear();
                                catalogue.page.set_neq(1);
                                Self::fetch_mangas(catalogue.clone());
                            }
                        }))
                    }))
                } else {
                    Some(html!("button", {
                        .text("Search")
                        .event(clone!(catalogue => move |_: events::Click| {
                            catalogue.is_search.set_neq(true);
                        }))
                    }))
                }
            })))
        })
    }

    pub fn render_search(catalogue: Rc<Self>) -> Dom {
        html!("div", {
            .class("search-box")
            .class([
                "w-full",
                "mb-2",
                "ml-0",
                "xl:ml-48",
                "p-2",
                "inline-flex",
                "transition-all",
            ])
            .visible_signal(catalogue.is_search.signal())
            .children(&mut [
                html!("input", {
                    .class([
                        "border",
                        "rounded",
                        "focus:outline-none",
                        "w-full",
                        "mr-2",
                        "p-1"
                    ])
                    .attribute("placeholder", "Search")
                    .attribute("type", "text")
                    .property_signal("value", catalogue.keyword.signal_cloned())
                    .event(clone!(catalogue => move |event: events::Input| {
                        catalogue.keyword.set_neq(event.value().unwrap_throw());
                    }))
                    .event_preventable(clone!(catalogue => move |event: events::KeyDown| {
                        if event.key() == "Enter" {
                            event.prevent_default();
                            catalogue.cover_list.lock_mut().clear();
                            catalogue.page.set_neq(1);
                            Self::fetch_mangas(catalogue.clone());
                        }
                    }))
                }),
                html!("button", {
                    .text("Cancel")
                    .event(clone!(catalogue => move |_: events::Click| {
                        catalogue.is_search.set_neq(false);
                        if catalogue.keyword.get_cloned() != "" {
                            catalogue.keyword.set_neq("".to_string());
                            catalogue.cover_list.lock_mut().clear();
                            catalogue.page.set_neq(1);
                            Self::fetch_mangas(catalogue.clone());
                        }
                    }))
                })
            ])
        })
    }

    pub fn render_main(catalogue: Rc<Self>) -> Dom {
        local_storage()
            .set("catalogue_id", catalogue.source_id.get().to_string().as_str())
            .unwrap_throw();
        html!("div", {
            .children(&mut [
                html!("div", {
                    .class("manga-grid")
                    .children_signal_vec(catalogue.cover_list.signal_vec_cloned().map(clone!(catalogue => move |cover| cover.render())))
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

    pub fn render_select(catalogue: Rc<Self>) -> Dom {
        html!("ul", {
            .class("list")
            .children_signal_vec(catalogue.sources.signal_vec_cloned().map(clone!(catalogue => move |source| html!("li", {
                .class("list-item")
                .children(&mut [
                    link!(Route::Catalogue{id: source.id, latest: false}.url(), {
                        .class("source-item")
                        .children(&mut [
                            html!("img", {
                                .class_signal("invisible", Mutable::new(source.icon.clone()).signal_cloned().map(|icon| icon.is_empty()))
                                .attribute("src", &source.icon)
                            }),
                            html!("span", {
                                .text(&source.name)
                            }),
                        ])
                    }),
                    link!(Route::Catalogue{id: source.id, latest: true}.url(), {
                        .class("source-action")
                        .text("latest")
                    }),
                ])
            }))))
        })
    }

    pub fn render(catalogue: Rc<Self>, source_id: i64, latest: bool) -> Dom {
        let saved_source_id = local_storage()
            .get("catalogue_id")
            .unwrap_throw()
            .unwrap_or("0".to_string())
            .parse::<i64>()
            .unwrap_or(0);

        if source_id > 0 && source_id != saved_source_id {
            local_storage()
                .set("catalogue_id", &source_id.to_string())
                .unwrap_throw();
            catalogue.source_id.set(source_id);
            let (sort_by, sort_order) = if latest {
                (SortByParam::LAST_UPDATED, SortOrderParam::DESC)
            } else {
                (SortByParam::VIEWS, SortOrderParam::DESC)
            };
            catalogue.sort_by.set(sort_by);
            catalogue.sort_order.set(sort_order);
            Self::fetch_mangas(catalogue.clone());
        } else if source_id == 0 {
            catalogue.reset();
            Self::fetch_sources(catalogue.clone());
        }

        html!("div", {
            .class("page")
            .children(&mut [
                Self::render_topbar(catalogue.clone()),
                html!("div", {
                    .class("topbar-spacing")
                })
            ])
            .child_signal(catalogue.source_id.signal().map(clone!(catalogue => move |x| if x > 0 {
                Some(Self::render_main(catalogue.clone()))
            } else {
                Some(Self::render_select(catalogue.clone()))
            })))
            .children(&mut [
                html!("div", {
                    .class("bottombar-spacing")
                })
            ])
        })
    }
}
