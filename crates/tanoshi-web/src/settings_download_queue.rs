use crate::{
    common::{DownloadQueueState, events, snackbar},
    query,
    utils::AsyncLoader,
};
use dominator::{Dom, clone, html, svg};

use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::SignalVecExt,
};
use gloo_timers::future::TimeoutFuture;
use std::rc::Rc;

pub struct SettingsDownloads {
    status: Mutable<bool>,
    queue: DownloadQueueState,
}

impl SettingsDownloads {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            status: Mutable::new(false),
            queue: DownloadQueueState::default(),
        })
    }

    fn pause_download(self: &Rc<Self>) {
        AsyncLoader::new().load({
            let settings = self.clone();
            async move {
                match query::pause_download().await {
                    Ok(status) => {
                        settings.status.set(status);
                    }
                    Err(err) => {
                        snackbar::show(format!("{err}"));
                    }
                }

                match query::download_status().await {
                    Ok(status) => {
                        settings.status.set(status);
                    }
                    Err(err) => {
                        snackbar::show(format!("{err}"));
                    }
                }
            }
        });
    }

    fn resume_download(self: &Rc<Self>) {
        AsyncLoader::new().load({
            let settings = self.clone();
            async move {
                match query::resume_download().await {
                    Ok(status) => {
                        settings.status.set(status);
                    }
                    Err(err) => {
                        snackbar::show(format!("{err}"));
                    }
                }

                match query::download_status().await {
                    Ok(status) => {
                        settings.status.set(status);
                    }
                    Err(err) => {
                        snackbar::show(format!("{err}"));
                    }
                }
            }
        });
    }

    fn remove_chapter_from_queue(self: &Rc<Self>, id: i64) {
        AsyncLoader::new().load(async move {
            if let Err(err) = query::remove_chapter_from_queue(&[id]).await {
                snackbar::show(format!("{err}"));
            }
        });
    }

    async fn watch_download_queue(self: Rc<Self>) {
        let mut retry_ms = 250;
        loop {
            self.queue.disconnected();
            let result = query::subscribe_download_queue(|update| {
                let status = update.download_status;
                let applied = self.queue.apply(update);
                if applied {
                    self.status.set_neq(status);
                    retry_ms = 250;
                }
                applied
            })
            .await;
            if let Err(error) = result {
                log::warn!("Download queue subscription: {error}");
            }
            TimeoutFuture::new(retry_ms).await;
            retry_ms = (retry_ms * 2).min(10_000);
        }
    }

    fn move_chapter(self: &Rc<Self>, chapter_id: i64, up: bool) {
        AsyncLoader::new().load(async move {
            if let Err(err) = query::move_chapter_in_queue(chapter_id, up).await {
                snackbar::show(format!("{err}"));
            }
        });
    }

    pub fn render(settings: Rc<Self>) -> Dom {
        html!("div", {
            .class("content")
            .future(settings.clone().watch_download_queue())
            .children(&mut [
                html!("div",{
                    .style("font-size", "smaller")
                    .style("display", "flex")
                    .style("justify-content", "flex-end")
                    .child_signal(settings.status.signal().map(clone!(settings => move |status| {
                        if status {
                            Some(html!("button", {
                                .attr("id", "select-all")
                                .style("display", "flex")
                                .style("align-items", "center")
                                .children(&mut [
                                    svg!("svg", {
                                        .attr("xmlns", "http://www.w3.org/2000/svg")
                                        .attr("viewBox", "0 0 20 20")
                                        .attr("fill", "currentColor")
                                        .class("icon")
                                        .children(&mut [
                                            svg!("path", {
                                                .attr("fill-rule", "evenodd")
                                                .attr("d", "M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z")
                                                .attr("clip-rule", "evenodd")
                                            })
                                        ])
                                    }),
                                    html!("span", {
                                        .style("margin", "0.25rem")
                                        .text("Pause")
                                    })
                                ])
                                .event(clone!(settings => move |_:events::Click| {
                                    settings.pause_download();
                                }))
                            }))
                        } else {
                            Some(html!("button", {
                                .attr("id", "select-all")
                                .style("display", "flex")
                                .style("align-items", "center")
                                .children(&mut [
                                    svg!("svg", {
                                        .attr("xmlns", "http://www.w3.org/2000/svg")
                                        .attr("viewBox", "0 0 20 20")
                                        .attr("fill", "currentColor")
                                        .class("icon")
                                        .children(&mut [
                                            svg!("path", {
                                                .attr("fill-rule", "evenodd")
                                                .attr("d", "M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z")
                                                .attr("clip-rule", "evenodd")
                                            }),
                                        ])
                                    }),
                                    html!("span", {
                                        .style("margin", "0.25rem")
                                        .text("Resume")
                                    })
                                ])
                                .event(clone!(settings => move |_:events::Click| {
                                    settings.resume_download();
                                }))
                            }))
                        }
                    })))
                }),
                html!("ul", {
                    .class("list")
                    .children_signal_vec(settings.queue.rows.signal_vec_cloned().map(clone!(settings => move |queue|
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
                                            .attr("id", "move-up-btn")
                                            .style("margin-top", "0.175rem")
                                            .style("margin-bottom", "0.175rem")
                                            .children(&mut [
                                                svg!("svg", {
                                                    .attr("xmlns", "http://www.w3.org/2000/svg")
                                                    .attr("viewBox", "0 0 20 20")
                                                    .attr("fill", "currentColor")
                                                    .class("icon")
                                                    .children(&mut [
                                                        svg!("path", {
                                                            .attr("fill-rule", "evenodd")
                                                            .attr("d", "M14.707 12.707a1 1 0 01-1.414 0L10 9.414l-3.293 3.293a1 1 0 01-1.414-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 010 1.414z")
                                                            .attr("clip-rule", "evenodd")
                                                        })
                                                    ])
                                                })
                                            ])
                                            .event(clone!(settings, queue => move |_:events::Click| {
                                                settings.move_chapter(queue.chapter_id, true);
                                            }))
                                        }),
                                        html!("button", {
                                            .attr("id", "remove-btn")
                                            .style("margin-top", "0.175rem")
                                            .style("margin-bottom", "0.175rem")
                                            .style("color", "red")
                                            .children(&mut [
                                                svg!("svg", {
                                                    .attr("xmlns", "http://www.w3.org/2000/svg")
                                                    .attr("viewBox", "0 0 20 20")
                                                    .attr("fill", "currentColor")
                                                    .class("icon")
                                                    .children(&mut [
                                                        svg!("path", {
                                                            .attr("fill-rule", "evenodd")
                                                            .attr("d", "M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z")
                                                            .attr("clip-rule", "evenodd")
                                                        })
                                                    ])
                                                })
                                            ])
                                            .event(clone!(settings, queue => move |_:events::Click| {
                                                settings.remove_chapter_from_queue(queue.chapter_id);
                                            }))
                                        }),
                                        html!("button", {
                                            .attr("id", "move-down-btn")
                                            .style("margin-top", "0.175rem")
                                            .style("margin-bottom", "0.175rem")
                                            .children(&mut [
                                                svg!("svg", {
                                                    .attr("xmlns", "http://www.w3.org/2000/svg")
                                                    .attr("viewBox", "0 0 20 20")
                                                    .attr("fill", "currentColor")
                                                    .class("icon")
                                                    .children(&mut [
                                                        svg!("path", {
                                                            .attr("fill-rule", "evenodd")
                                                            .attr("d", "M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z")
                                                            .attr("clip-rule", "evenodd")
                                                        })
                                                    ])
                                                })
                                            ])
                                            .event(clone!(settings, queue => move |_:events::Click| {
                                                settings.move_chapter(queue.chapter_id, false);
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
                                                    .text_signal(queue.data.signal_ref(|data| data.manga_title.clone()))
                                                }),
                                                html!("span", {
                                                    .text_signal(queue.data.signal_ref(|data| data.source_name.clone()))
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
                                                    .text_signal(queue.data.signal_ref(|data| data.chapter_title.clone()))
                                                }),
                                                html!("span", {
                                                    .text_signal(queue.data.signal_ref(|data| format!("{}/{}", data.downloaded, data.total)))
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
                                                    .style_signal("width", queue.data.signal_ref(|data| format!("{}%", if data.total > 0 { data.downloaded as f64 / data.total as f64 * 100.0 } else { 0.0 })))
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
