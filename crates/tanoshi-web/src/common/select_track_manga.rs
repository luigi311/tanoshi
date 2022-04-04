use crate::{
    common::{snackbar, Modal, Spinner},
    query,
    utils::AsyncLoader,
};
use chrono::NaiveDateTime;
use dominator::{clone, events, html, with_node, Dom, EventOptions};
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::{MutableVec, SignalVecExt},
};
use std::rc::Rc;
use web_sys::HtmlInputElement;

#[derive(Debug, Default, Clone)]
pub struct TrackerStatus {
    pub tracker: String,
    pub tracker_manga_id: Mutable<Option<String>>,
    pub tracker_manga_title: Mutable<Option<String>>,
    pub status: Mutable<Option<String>>,
    pub score: Mutable<Option<i64>>,
    pub num_chapters_read: Mutable<Option<i64>>,
    pub start_date: Mutable<Option<NaiveDateTime>>,
    pub finish_date: Mutable<Option<NaiveDateTime>>,
}

#[derive(Debug, Clone)]
pub enum State {
    SelectTracker,
    SelectManga(TrackerStatus),
}

#[derive(Debug, Clone)]
pub struct TrackerManga {
    pub tracker: String,
    pub tracker_manga_id: String,
    pub title: String,
    pub synopsis: String,
    pub cover_url: String,
}

pub struct SelectTrackMangaModal {
    manga_id: i64,
    state: Mutable<State>,
    trackers: MutableVec<TrackerStatus>,
    title: Mutable<String>,
    manga_list: MutableVec<TrackerManga>,
    selected_tracker_manga: Mutable<Option<TrackerManga>>,
    modal: Rc<Modal>,
    loader: AsyncLoader,
}

impl SelectTrackMangaModal {
    pub fn new(manga_id: i64, title: String) -> Rc<Self> {
        Rc::new(Self {
            manga_id,
            state: Mutable::new(State::SelectTracker),
            trackers: MutableVec::new(),
            title: Mutable::new(title),
            manga_list: MutableVec::new(),
            selected_tracker_manga: Mutable::new(None),
            modal: Modal::new_with_default(true),
            loader: AsyncLoader::new(),
        })
    }

    pub fn fetch_manga_tracker_status(self: &Rc<Self>) {
        let select = self.clone();
        self.loader.load(clone!(select => async move {
            match query::fetch_manga_tracker_status(select.manga_id).await {
                Ok(res) => {
                    select.trackers.lock_mut().replace_cloned(res.iter().map(|tracker| TrackerStatus { 
                        tracker: tracker.tracker.to_owned(), 
                        tracker_manga_id: Mutable::new(tracker.tracker_manga_id.clone()), 
                        tracker_manga_title: Mutable::new(tracker.tracker_manga_title.clone()), 
                        status: Mutable::new(tracker.status.clone()), 
                        score: Mutable::new(tracker.score.clone()), 
                        num_chapters_read: Mutable::new(tracker.num_chapters_read.clone()), 
                        start_date: Mutable::new(tracker.start_date.as_ref().and_then(|date| NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%.f").ok())), 
                        finish_date: Mutable::new(tracker.finish_date.as_ref().and_then(|date| NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%.f").ok()))
                    }).collect())
                }
                Err(e) => {
                    snackbar::show(format!("failed to fetch manga tracker status {}", e));
                }
            }
        }));
    }

    pub fn fetch_manga_list(self: &Rc<Self>, tracker: &TrackerStatus) {
        let select = self.clone();
        let tracker = tracker.tracker.clone();
        let title = self.title.get_cloned();
        self.loader.load(clone!(select => async move {
            match query::search_tracker_manga(tracker.to_owned(), title).await {
                Ok(res) => {
                    select.manga_list.lock_mut().replace_cloned(res.into_iter().map(|m| TrackerManga{
                        tracker: tracker.to_owned(),
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
            if let Some((tracker, tracker_manga_id)) = select.selected_tracker_manga.get_cloned().map(|tracker| (tracker.tracker.clone(), tracker.tracker_manga_id)) {
                match query::track_manga(select.manga_id, tracker, tracker_manga_id).await {
                    Ok(_) => {}
                    Err(e) => {
                        snackbar::show(format!("failed to add manga to tracker {}", e));
                    }
                }
            }
        }));
    }

    pub fn untrack_manga(self: &Rc<Self>, manga_id: i64, tracker: String) {
        let select = self.clone();
        self.loader.load(clone!(select => async move {
            match query::untrack_manga(manga_id, tracker).await {
                Ok(_) => {
                    select.fetch_manga_tracker_status();
                }
                Err(e) => {
                    snackbar::show(format!("failed to add manga to tracker {}", e));
                }
            }
        }));
    }

    pub fn render_header<F>(self: &Rc<Self>, f: F) -> Dom where F: Fn() + Clone + 'static {
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
                    .event(clone!(select, f => move |_: events::Click| {
                        select.track_manga();
                        f();
                    }))
                })
            ])
        })
    }

    pub fn render_manga_list(self: &Rc<Self>, tracker: &TrackerStatus) -> Dom {
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
                    .children_signal_vec(select.manga_list.signal_vec_cloned().map(clone!(select => move |manga| {
                        html!("div", {
                            .style("display", "flex")
                            .style("padding", "0.5rem")
                            .style("border-radius", "0.5rem")
                            .style_signal("border", select.selected_tracker_manga.signal_cloned().map(clone!(manga => move |selected_tracker| {
                                info!("{selected_tracker:?}");
                                selected_tracker.and_then(|selected_tracker| (selected_tracker.tracker == manga.tracker && selected_tracker.tracker_manga_id == manga.tracker_manga_id).then(|| "var(--list-group-border)"))
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
                            .event(clone!(select, manga => move |_: events::Click| {
                                select.selected_tracker_manga.set(Some(manga.clone()));
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
            .children_signal_vec(select.trackers.signal_vec_cloned().map(clone!(select => move |tracker| html!("div", {
                .children(&mut [
                    html!("div", {
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
                                ])
                                .child_signal(tracker.tracker_manga_title.signal_cloned().map(clone!(tracker => move |tracker_manga_title| if let Some(tracker_manga_title) = tracker_manga_title {
                                    Some(html!("span", {
                                        .style("margin-left", "0.5rem")
                                        .text(&tracker_manga_title)
                                    }))
                                } else {
                                    Some(html!("span", {
                                        .style("margin-left", "0.5rem")
                                        .text(&tracker.tracker)
                                    }))
                                })))
                            }),
                        ])
                        .child_signal(tracker.tracker_manga_id.signal_cloned().map(clone!(select, tracker => move |tracker_manga_id| if tracker_manga_id.is_some() {
                            Some(html!("button", {
                                .style("color", "red")
                                .text("Remove")
                                .event(clone!(select, tracker => move |_: events::Click| {
                                    select.untrack_manga(select.manga_id, tracker.tracker.clone());
                                }))
                            }))
                        } else {
                            Some(html!("button", {
                                .text("Add")
                                .event(clone!(select, tracker => move |_: events::Click| {
                                    select.state.set(State::SelectManga(tracker.clone()));
                                }))
                            }))
                        })))
                    })
                ])
            }))))
        })
    }

    pub fn render<F>(self: &Rc<Self>, f: F) -> Dom where F: Fn() + Clone + 'static {
        info!("render select track");
        let select = self.clone();
        self.fetch_manga_tracker_status();
        self.modal.render(
            &mut [html!("div", {
                .child_signal(select.state.signal_cloned().map(clone!(select => move |state| match state {
                    State::SelectTracker => {
                        None
                    }
                    State::SelectManga(_) => {
                        Some(select.render_header(f.clone()))
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
                .child_signal(select.loader.is_loading().map(|is_loading| is_loading.then(|| Spinner::render_spinner(true))))
            })],
        )
    }
}
