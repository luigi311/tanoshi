use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

use dominator::{Dom, EventOptions, clone, events, html, routing};
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::MutableVec,
};
use serde::{Deserialize, Serialize};

use crate::{
    common::{Cover, Route, Spinner, snackbar},
    migration, query,
    utils::AsyncLoader,
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct SourceLite {
    id: i64,
    name: String,
    icon: String,
}

pub struct SettingsMigrate {
    loader: AsyncLoader,
    spinner: Rc<Spinner>,

    // source_id -> source meta
    sources: MutableVec<SourceLite>,

    // source_id -> covers
    grouped: Mutable<BTreeMap<i64, Vec<Cover>>>,

    // Which manga are we migrating
    selected_cover: Mutable<Option<Cover>>,
}

impl SettingsMigrate {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            loader: AsyncLoader::new(),
            spinner: Spinner::new(),
            sources: MutableVec::new(),
            grouped: Mutable::new(BTreeMap::new()),
            selected_cover: Mutable::new(None),
        })
    }

    fn open_picker(&self, cover: Cover) {
        debug!("Selected cover {} for migration", cover.title);
        self.selected_cover.set(Some(cover));
    }

    fn close_picker(&self) {
        self.selected_cover.set(None);
    }

    fn go_to_destination(&self, dest_source_id: i64, cover: &Cover) {
        debug!(
            "Going to /catalogue/{} with migration context",
            dest_source_id
        );
        // Save migration context so Catalogue knows we're selecting a target
        migration::set(&migration::MigrationState {
            from_manga_id: cover.id,
            from_source_id: cover.source_id,
            from_title: cover.title.clone(),
            to_source_id: dest_source_id,
        });

        // Pre-fill catalogue search with the title
        let url = Route::Catalogue {
            id: dest_source_id,
            latest: false,
            query: Some(cover.title.clone()),
        }
        .url();

        self.close_picker();

        routing::go_to_url(&url);
    }

    pub fn fetch(settings: Rc<Self>) {
        settings.spinner.set_active(true);

        settings.loader.load(clone!(settings => async move {
            // 1) Fetch sources (for names/icons)
            match query::fetch_sources().await {
                Ok(res) => {
                    let sources: Vec<SourceLite> = res
                        .iter()
                        .map(|s| SourceLite {
                            id: s.id,
                            name: s.name.clone(),
                            icon: s.icon.clone(),
                        })
                        .collect();

                    settings.sources.lock_mut().replace_cloned(sources);
                }
                Err(e) => snackbar::show(format!("Failed to fetch sources: {e}")),
            }

            // 2) Fetch the full library and group by source_id. library(categoryId: null)
            // only returns uncategorized manga, so every category has to be fetched too.
            let mut category_ids: Vec<Option<i64>> = vec![None];
            match query::fetch_categories().await {
                Ok(categories) => {
                    category_ids.extend(categories.iter().filter_map(|c| c.id).map(Some));
                }
                Err(e) => snackbar::show(format!("Failed to fetch categories: {e}")),
            }

            let mut map: BTreeMap<i64, Vec<Cover>> = BTreeMap::new();
            let mut seen: HashSet<i64> = HashSet::new();

            let futures: Vec<_> = category_ids
                .iter()
                .map(|&category_id| query::fetch_manga_from_favorite(category_id))
                .collect();
            let results = futures::future::join_all(futures).await;
            for result in results {
                match result {
                    Ok(covers) => {
                        for c in covers {
                            if seen.insert(c.id) {
                                map.entry(c.source_id).or_default().push(c);
                            }
                        }
                    }
                    Err(e) => snackbar::show(format!("Failed to fetch library: {e}")),
                }
            }
            for covers in map.values_mut() {
                covers.sort_by(|a, b| a.title.cmp(&b.title));
            }
            settings.grouped.set(map);

            settings.spinner.set_active(false);
        }));
    }

    fn source_name(sources: &Vec<SourceLite>, source_id: i64) -> String {
        sources
            .iter()
            .find(|s| s.id == source_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| format!("Source {source_id}"))
    }

    pub fn render(self: Rc<Self>) -> Dom {
        // load once on first render
        if self.sources.lock_ref().is_empty() && self.grouped.get_cloned().is_empty() {
            Self::fetch(self.clone());
        }

        html!("div", {
            .class("content")
            .style("padding", "0.5rem")
            .children(&mut [
                html!("div", {
                    .style("margin-bottom", "0.5rem")
                    .children(&mut [
                        html!("h2", {.text("Migrate Manga")}),
                        html!("div", {
                            .style("font-size", "0.875rem")
                            .style("opacity", "0.8")
                            .text("Pick a manga from your library, then choose the destination source and manga. Categories, read progress, and trackers are carried over.")
                        }),
                    ])
                }),
                Spinner::render(self.spinner.clone()),
            ])
            .child_signal(self.grouped.signal_cloned().map({
                let migrate = self.clone();
                move |grouped| {
                    let sources_vec = migrate.sources.lock_ref().to_vec();
                    Some(html!("div", {
                        .children(&mut grouped.into_iter().map(|(source_id, covers)| {
                            let source_name = Self::source_name(&sources_vec, source_id);

                            html!("div", {
                                .style("margin-bottom", "1rem")
                                .children(&mut [
                                    html!("div", {
                                        .style("display", "flex")
                                        .style("justify-content", "space-between")
                                        .style("align-items", "center")
                                        .style("margin", "0.25rem 0")
                                        .children(&mut [
                                            html!("h3", { .text(&source_name) }),
                                            html!("span", {
                                                .style("font-size", "0.875rem")
                                                .style("opacity", "0.8")
                                                .text(format!("{}", covers.len()).as_str())
                                            })
                                        ])
                                    }),
                                    html!("ul", {
                                        .class(["list", "group"])
                                        .children(&mut covers.into_iter().map(|cover| {
                                            html!("li", {
                                                .class("list-item")
                                                .style("display", "flex")
                                                .style("justify-content", "space-between")
                                                .style("align-items", "center")
                                                .children(&mut [
                                                    html!("span", {
                                                        .style("overflow", "hidden")
                                                        .style("text-overflow", "ellipsis")
                                                        .style("white-space", "nowrap")
                                                        .text(&cover.title)
                                                    }),
                                                    html!("button", {
                                                        .text("Migrate")
                                                        .event_with_options(&EventOptions::preventable(),
                                                            clone!(migrate, cover => move |e: events::Click| {
                                                                e.stop_propagation();
                                                                migrate.open_picker(cover.clone());
                                                            })
                                                        )
                                                    })
                                                ])
                                            })
                                        }).collect::<Vec<_>>())
                                    })
                                ])
                            })
                        }).collect::<Vec<_>>())
                    }))
                }
            }))
            .child_signal(self.selected_cover.signal_cloned().map({
                let migrate = self.clone();
                move |selected| {
                    selected.map(|cover| {
                        // snapshot the sources list (cheap + avoids holding locks in closures)
                        let sources = migrate.sources.lock_ref().to_vec();
                        let current_source_id = cover.source_id;

                        html!("div", {
                            // Backdrop (visual only)
                            .style("position", "fixed")
                            .style("inset", "0")
                            .style("background", "rgba(0,0,0,0.5)")
                            .style("z-index", "9999")

                            .child(html!("div", {
                                // Modal card
                                .style("background", "var(--modal-background-color)")
                                .style("color", "var(--color)")
                                .style("box-shadow", "var(--shadow)")
                                .style("border-radius", "0.75rem")
                                .style("max-width", "32rem")
                                .style("margin", "10vh auto")
                                .style("padding", "1rem")
                                .children(&mut [
                                    html!("h3", {
                                        .style("margin", "0 0 0.25rem 0")
                                        .text("Choose destination source")
                                    }),
                                    html!("div", {
                                        .style("font-size", "0.9rem")
                                        .style("opacity", "0.8")
                                        .text(format!("Migrate: {}", cover.title).as_str())
                                    }),
                                    html!("div", { .style("height", "0.75rem") }),
                                    html!("ul", {
                                        .class(["list", "group"])
                                        .children(&mut sources.into_iter()
                                            .filter(|s| s.id != current_source_id)
                                            .map(|s| {
                                                html!("li", {
                                                    .class("list-item")
                                                    .style("display", "flex")
                                                    .style("justify-content", "space-between")
                                                    .style("align-items", "center")
                                                    .children(&mut [
                                                        html!("div", {
                                                            .style("display", "flex")
                                                            .style("align-items", "center")
                                                            .children(&mut [
                                                                html!("img", {
                                                                    .style("width", "1.75rem")
                                                                    .style("height", "1.75rem")
                                                                    .style("margin-right", "0.5rem")
                                                                    .attr("src", &s.icon)
                                                                    .attr("alt", "")
                                                                }),
                                                                html!("span", { .text(&s.name) }),
                                                            ])
                                                        }),
                                                        html!("button", {
                                                            .text("Select")
                                                            .event(clone!(migrate, cover => move |_: events::Click| {
                                                                migrate.go_to_destination(s.id, &cover);
                                                            }))
                                                        })
                                                    ])
                                                })
                                            })
                                            .collect::<Vec<_>>()
                                        )
                                    }),
                                    html!("div", {
                                        .style("display", "flex")
                                        .style("justify-content", "flex-end")
                                        .style("margin-top", "0.75rem")
                                        .child(html!("button", {
                                            .text("Cancel")
                                            .event(clone!(migrate => move |_: events::Click| {
                                                migrate.close_picker();
                                            }))
                                        }))
                                    })
                                ])
                            }))
                        })
                    })
                }
            }))
        })
    }
}
