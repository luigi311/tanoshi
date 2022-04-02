use crate::{
    common::{snackbar, Modal},
    query,
    utils::AsyncLoader,
};
use dominator::{clone, events, html, with_node, Dom, EventOptions};
use futures_signals::{
    signal::{Mutable, SignalExt, self},
    signal_vec::{self, MutableVec, SignalVecExt},
};
use std::rc::Rc;
use web_sys::HtmlInputElement;

#[derive(Debug, Clone)]
pub struct Tracker {
    pub tracker: String,
    pub tracker_manga_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum State {
    SelectTracker,
    SelectManga(Tracker),
}

#[derive(Debug, Clone)]
pub struct TrackerManga {
    pub tracker_manga_id: String,
    pub title: String,
    pub synopsis: String,
    pub cover_url: String,
}

pub struct SelectTrackMangaModal {
    manga_id: i64,
    state: Mutable<State>,
    trackers: Vec<Tracker>,
    title: Mutable<String>,
    manga_list: MutableVec<TrackerManga>,
    selected_tracker: Mutable<Option<Tracker>>,
    modal: Rc<Modal>,
    loader: AsyncLoader,
}

impl SelectTrackMangaModal {
    pub fn new(trackers: Vec<Tracker>, manga_id: i64, title: String) -> Rc<Self> {
        Rc::new(Self {
            manga_id,
            state: Mutable::new(State::SelectTracker),
            trackers,
            title: Mutable::new(title),
            manga_list: MutableVec::new(),
            selected_tracker: Mutable::new(None),
            modal: Modal::new_with_default(true),
            loader: AsyncLoader::new(),
        })
    }

    pub fn fetch_manga_list(self: &Rc<Self>, tracker: &Tracker) {
        let select = self.clone();
        let tracker = tracker.tracker.clone();
        let title = self.title.get_cloned();
        self.loader.load(clone!(select => async move {
            match query::search_tracker_manga(tracker, title).await {
                Ok(res) => {
                    select.manga_list.lock_mut().replace_cloned(res.into_iter().map(|m| TrackerManga{
                        tracker_manga_id: m.tracker_manga_id,
                        title: m.title,
                        cover_url: m.cover_url,
                        synopsis: m.synopsis
                    }).collect());
                }
                Err(e) => {
                    snackbar::show(format!("failed to fetch tracker manga list {}", e));
                }
            }
        }));
    }

    pub fn track_manga(self: &Rc<Self>) {
        let select = self.clone();
        self.loader.load(clone!(select => async move {
            if let Some((tracker, tracker_manga_id)) = select.selected_tracker.get_cloned().and_then(|tracker| tracker.tracker_manga_id.as_ref().map(|id| (tracker.tracker.clone(), id.clone()))) {
                match query::track_manga(select.manga_id, tracker, tracker_manga_id).await {
                    Ok(_) => {
                        select.modal.toggle_show();
                    }
                    Err(e) => {
                        snackbar::show(format!("failed to add manga to tracker {}", e));
                    }
                }
            }
            
        }));
    }

    pub fn render_header(self: &Rc<Self>) -> Dom {
        let select = self.clone();
        html!("div", {
            .style("display", "flex")
            .style("justify-content", "space-between")
            .style("margin-bottom", "0.5rem")
            .children(&mut [
                html!("span", {
                    .style("font-size", "large")
                    .text("Select Manga")
                }),
                html!("button", {
                    .text("Track")
                    .event(clone!(select => move |_: events::Click| {
                        select.track_manga();
                    }))
                })
            ])
        })
    }

    pub fn render_manga_list(self: &Rc<Self>, tracker: &Tracker) -> Dom {
        let select = self.clone();

        html!("div", {
            .style("display", "flex")
            .style("flex-direction", "column")
            .future(self.title.signal_cloned().for_each(clone!(select, tracker => move |_| {
                select.fetch_manga_list(&tracker);

                async {}
            })))
            .children(&mut [
                html!("input" => HtmlInputElement, {
                    .attribute("type", "text")
                    .attribute("placeholder", "Manga title")
                    .property("value", &self.title.get_cloned())
                    .with_node!(input => {
                        .event_with_options(&EventOptions::preventable(), clone!(select => move |e: events::KeyDown| {
                            if e.key() == "Enter" {
                                e.prevent_default();
                                select.title.set(input.value());
                            }
                        }))
                    })
                }),
                html!("ul", {
                    .class("list")
                    .style("height", "500px")
                    .style("overflow-y", "auto")
                    .children_signal_vec(select.manga_list.signal_vec_cloned().map(clone!(select, tracker => move |manga| {
                        html!("div", {
                            .style("display", "flex")
                            .style("padding", "0.5rem")
                            .style("border-radius", "0.5rem")
                            .style_signal("border", select.selected_tracker.signal_cloned().map(clone!(manga, tracker => move |selected_tracker| if let Some(selected_tracker) = selected_tracker {
                                if selected_tracker.tracker == tracker.tracker && selected_tracker.tracker_manga_id == Some(manga.tracker_manga_id.clone()) {
                                    Some("var(--list-group-border)")
                                } else {
                                    None
                                }
                            } else {
                                None
                            })))
                            .children(&mut [
                                html!("div", {
                                    .children(&mut [
                                        html!("img", {
                                            .style("border-radius", "0.375rem")
                                            .style("border-width", "0.375rem")
                                            .style("width", "5rem")
                                            .style("height", "auto")
                                            .attribute("src", &manga.cover_url)
                                        }),
                                    ])
                                }),
                                html!("div", {
                                    .style("display", "flex")
                                    .style("flex-direction", "column")
                                    .style("margin-left", "0.5rem")
                                    .children(&mut [
                                        html!("span", {
                                            .style("margin-bottom", "0.5rem")
                                            .style("font-weight", "bold")
                                            .text(&manga.title)
                                        }),
                                        html!("span", {
                                            .style("display", "-webkit-box")
                                            .style("-webkit-line-clamp", "5")
                                            .style("-webkit-box-orient", "vertical")
                                            .style("overflow", "hidden")
                                            .text(&manga.synopsis)
                                        })
                                    ])
                                })
                            ])
                            .event(clone!(select, tracker, manga => move |_: events::Click| {
                                select.selected_tracker.set(Some(Tracker{
                                    tracker: tracker.tracker.clone(),
                                    tracker_manga_id: Some(manga.tracker_manga_id.clone())
                                }));
                            }))
                        })
                    })))
                })
            ])
        })
    }

    pub fn render_main(self: &Rc<Self>) -> Dom {
        let select = self.clone();
        html!("ul", {
            .class("list")
            .children_signal_vec(signal_vec::always(select.trackers.clone()).map(clone!(select => move |tracker| html!("div", {
                .style("padding", "0.5rem")
                .style("display", "flex")
                .style("justify-content", "space-between")
                .style("align-items", "center")
                .children(&mut [
                    html!("div", {
                        .style("display", "flex")
                        .style("align-items", "center")
                        .children(&mut [
                            html!("img", {
                                .style("width", "20px")
                                .style("height", "20px")
                                .attribute("src", if tracker.tracker == "myanimelist" { "https://myanimelist.net/img/common/pwa/launcher-icon-0-75x.png" } else { "" })
                            }),
                            html!("span", {
                                .style("margin-left", "0.5rem")
                                .text(&tracker.tracker)
                            })
                        ])
                    }),
                ])
                .child_signal(signal::always(tracker.tracker_manga_id.clone()).map(clone!(select, tracker => move |tracker_manga_id| if let Some(tracker_manga_id) = tracker_manga_id {
                    Some(html!("span", {
                        .text(&format!("{tracker_manga_id}"))
                    }))
                } else {
                    Some(html!("button", {
                        .text("Add")
                        .event(clone!(select, tracker => move |_: events::Click| {
                            select.state.set(State::SelectManga(tracker.clone()));
                        }))
                    }))
                })))
            }))))
        })
    }

    pub fn render(self: &Rc<Self>) -> Dom {
        info!("render select track");
        let select = self.clone();
        self.modal.render(
            &mut [html!("div", {
                .child_signal(select.state.signal_cloned().map(clone!(select => move |state| match state {
                    State::SelectTracker => {
                        None
                    }
                    State::SelectManga(_) => {
                        Some(select.render_header())
                    }
                })))
                .child_signal(select.state.signal_cloned().map(clone!(select => move |state| match state {
                    State::SelectTracker => {
                        Some(select.render_main())
                    }
                    State::SelectManga(tracker) => {
                        Some(select.render_manga_list(&tracker))
                    }
                })))
            })],
        )
    }
}
