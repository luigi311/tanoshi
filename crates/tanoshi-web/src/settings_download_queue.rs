use crate::{
    common::{events, snackbar, DownloadQueue},
    query,
    utils::AsyncLoader,
};
use dominator::{clone, html, svg, Dom};

use futures_signals::{signal::Mutable, signal_vec::MutableVec, signal_vec::SignalVecExt};
use gloo_timers::callback::Interval;
use std::rc::Rc;

pub struct SettingsDownloads {
    queue: MutableVec<DownloadQueue>,
    interval: Mutable<Option<Interval>>,
    loader: AsyncLoader,
}

impl SettingsDownloads {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            queue: MutableVec::new(),
            interval: Mutable::new(None),
            loader: AsyncLoader::new(),
        })
    }

    fn fetch_download_queue(self: &Rc<Self>) {
        self.loader.load({
            let settings = self.clone();
            async move {
                match query::fetch_download_queue().await {
                    Ok(data) => {
                        let queue = data
                            .iter()
                            .map(|queue| DownloadQueue {
                                source_name: queue.source_name.clone(),
                                manga_title: queue.manga_title.clone(),
                                chapter_id: queue.chapter_id,
                                chapter_title: queue.chapter_title.clone(),
                                downloaded: queue.downloaded,
                                total: queue.total,
                                priority: queue.priority,
                            })
                            .collect();
                        settings.queue.lock_mut().replace_cloned(queue);
                    }
                    Err(err) => {
                        snackbar::show(format!("{}", err));
                    }
                }
            }
        });
    }

    fn update_chapter_priority(self: &Rc<Self>, chapter_id: i64, priority: i64) {
        self.loader.load({
            let settings = self.clone();
            async move {
                match query::update_chapter_priority(chapter_id, priority).await {
                    Ok(_) => {
                        settings.fetch_download_queue();
                    }
                    Err(err) => {
                        snackbar::show(format!("{}", err));
                    }
                }
            }
        });
    }

    pub fn render(settings: Rc<Self>) -> Dom {
        settings.fetch_download_queue();
        html!("div", {
            .after_inserted(clone!(settings => move |_| {
                settings.interval.set(Some(Interval::new(1_000, clone!(settings => move || {
                    settings.fetch_download_queue();
                }))));
            }))
            .after_removed(clone!(settings => move |_| {
                if let Some(interval) = settings.interval.replace(None) {
                    interval.cancel();
                }
            }))
            .children(&mut [
                html!("ul", {
                    .class("list")
                    .children_signal_vec(settings.queue.signal_vec_cloned().map(clone!(settings => move |queue|
                        html!("li", {
                            .class("list-item")
                            .style("display", "flex")
                            .style("align-items", "center")
                            .children(&mut [
                                html!("div", {
                                    .style("display", "flex")
                                    .style("flex-direction", "column")
                                    .style("align-items", "center")
                                    .children(&mut [
                                        html!("button", {
                                            .attribute("id", "deselect-all")
                                            .style("margin", "0.25rem")
                                            .children(&mut [
                                                svg!("svg", {
                                                    .attribute("xmlns", "http://www.w3.org/2000/svg")
                                                    .attribute("viewBox", "0 0 20 20")
                                                    .attribute("fill", "currentColor")
                                                    .class("icon")
                                                    .children(&mut [
                                                        svg!("path", {
                                                            .attribute("fill-rule", "evenodd")
                                                            .attribute("d", "M14.707 12.707a1 1 0 01-1.414 0L10 9.414l-3.293 3.293a1 1 0 01-1.414-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 010 1.414z")
                                                            .attribute("clip-rule", "evenodd")
                                                        })
                                                    ])
                                                })
                                            ])
                                            .event(clone!(settings, queue => move |_:events::Click| {
                                                if queue.priority - 1 > 0 {
                                                    settings.update_chapter_priority(queue.chapter_id, queue.priority - 1);
                                                }
                                            }))
                                        }),
                                        html!("button", {
                                            .attribute("id", "deselect-all")
                                            .style("margin", "0.25rem")
                                            .children(&mut [
                                                svg!("svg", {
                                                    .attribute("xmlns", "http://www.w3.org/2000/svg")
                                                    .attribute("viewBox", "0 0 20 20")
                                                    .attribute("fill", "currentColor")
                                                    .class("icon")
                                                    .children(&mut [
                                                        svg!("path", {
                                                            .attribute("fill-rule", "evenodd")
                                                            .attribute("d", "M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z")
                                                            .attribute("clip-rule", "evenodd")
                                                        })
                                                    ])
                                                })
                                            ])
                                            .event(clone!(settings, queue => move |_:events::Click| {
                                                settings.update_chapter_priority(queue.chapter_id, queue.priority + 1);
                                            }))
                                        }),
                                    ])
                                }),
                                html!("div", {
                                    .style("display", "flex")
                                    .style("flex-direction", "column")
                                    .style("width", "100%")
                                    .style("margin", "0.25rem")
                                    .children(&mut [
                                        html!("div", {
                                            .style("display", "flex")
                                            .style("justify-content", "space-between")
                                            .style("width", "100%")
                                            .style("margin", "0.25rem")
                                            .children(&mut [
                                                html!("span", {
                                                    .style("font-weight", "600")
                                                    .text(&queue.manga_title)
                                                }),
                                                html!("span", {
                                                    .text(&queue.source_name)
                                                }),
                                            ])
                                        }),
                                        html!("div", {
                                            .style("display", "flex")
                                            .style("justify-content", "space-between")
                                            .style("width", "100%")
                                            .style("margin", "0.25rem")
                                            .children(&mut [
                                                html!("span", {
                                                    .text(&queue.chapter_title)
                                                }),
                                                html!("span", {
                                                    .text(&format!("{}/{}", queue.downloaded, queue.total))
                                                })
                                            ])
                                        }),
                                        html!("div", {
                                            .style("height", "0.5rem")
                                            .style("width", "100%")
                                            .style("margin", "0.25rem")
                                            .style("background-color", "var(--primary-color-300)")
                                            .children(&mut [
                                                html!("div", {
                                                    .style("width", &format!("{}%", (queue.downloaded as f64 / queue.total as f64) * 100.0))
                                                    .style("height", "100%")
                                                    .style("background-color", "var(--primary-color)")
                                                })
                                            ])
                                        })
                                    ])
                                })
                            ])
                        })
                    )))
                })
            ])
        })
    }
}
