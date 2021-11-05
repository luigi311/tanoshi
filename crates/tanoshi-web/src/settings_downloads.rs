use crate::{
    common::{snackbar, DownloadQueue},
    query,
    utils::AsyncLoader,
};
use dominator::{clone, html, Dom};

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
                match query::download_queue().await {
                    Ok(data) => {
                        let queue = data
                            .iter()
                            .map(|queue| DownloadQueue {
                                source_name: queue.source_name.clone(),
                                manga_title: queue.manga_title.clone(),
                                chapter_title: queue.chapter_title.clone(),
                                downloaded: queue.downloaded,
                                total: queue.total,
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
                    .class([
                        "list",
                        "group"
                    ])
                    .children_signal_vec(settings.queue.signal_vec_cloned().map(|queue|
                        html!("li", {
                            .class("list-item")
                            .style("display", "flex")
                            .style("flex-direction", "column")
                            .children(&mut [
                                html!("div", {
                                    .style("display", "flex")
                                    .style("justify-content", "space-between")
                                    .style("width", "100%")
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
                    ))
                })
            ])
        })
    }
}
