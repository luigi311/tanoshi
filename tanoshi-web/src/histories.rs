use std::rc::Rc;

use crate::query;
use crate::utils::{proxied_image_url, AsyncLoader};
use crate::{
    app::App,
    common::{Route, Spinner},
};
use dominator::{clone, events, html, link, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use wasm_bindgen::UnwrapThrowExt;

#[derive(Debug, Clone)]
pub struct Entry {
    manga_id: i64,
    manga_title: String,
    cover_url: String,
    chapter_id: i64,
    chapter_title: String,
    read_at: chrono::NaiveDateTime,
    last_page_read: i64,
    cursor: String,
}

pub struct Histories {
    loader: AsyncLoader,
    spinner: Rc<Spinner>,
    entries: MutableVec<Entry>,
    is_entries_empty: Mutable<bool>,
}

impl Histories {
    pub fn new() -> Rc<Self> {
        Rc::new(Histories {
            spinner: Spinner::new(),
            loader: AsyncLoader::new(),
            entries: MutableVec::new(),
            is_entries_empty: Mutable::new(true),
        })
    }

    pub fn fetch_read_histories(histories: Rc<Self>) {
        histories.spinner.set_active(true);
        histories.loader.load(clone!(histories => async move {
            let cursor = histories.entries.lock_ref().last().map(|entry| entry.cursor.clone());
            match query::fetch_histories(cursor).await {
                Ok(result) => {
                    for edge in result.edges.unwrap_throw() {
                        histories.entries.lock_mut().push_cloned(Entry{
                            manga_id: edge.as_ref().unwrap_throw().node.manga_id,
                            manga_title: edge.as_ref().unwrap_throw().node.manga_title.clone(),
                            cover_url: edge.as_ref().unwrap_throw().node.cover_url.clone(),
                            chapter_id: edge.as_ref().unwrap_throw().node.chapter_id,
                            chapter_title: edge.as_ref().unwrap_throw().node.chapter_title.clone(),
                            read_at: chrono::NaiveDateTime::parse_from_str(&edge.as_ref().unwrap_throw().node.read_at, "%Y-%m-%dT%H:%M:%S%.f").unwrap_throw(),
                            last_page_read: edge.as_ref().unwrap_throw().node.last_page_read,
                            cursor: edge.as_ref().unwrap_throw().cursor.clone(),
                        })
                    }
                    histories.is_entries_empty.set(histories.entries.lock_ref().is_empty());
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
            histories.spinner.set_active(false);
        }));
    }

    pub fn render_topbar() -> Dom {
        html!("div", {
            .class([
                "px-2",
                "pb-2",
                "flex",
                "justify-between",
                "fixed",
                "left-0",
                "right-0",
                "top-0",
                "z-50",
                "bg-accent",
                "dark:bg-gray-900",
                "border-b",
                "border-accent-darker",
                "dark:border-gray-800",
                "text-gray-50",
                "pt-safe-top"
            ])
            .children(&mut [
                html!("span", {
                    .class([
                        "mx-auto",
                        "text-gray-50"
                    ])
                    .text("Histories")
                })
            ])
        })
    }

    pub fn render_main(histories: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "px-2",
                "xl:pr-2",
                "xl:pl-52",
            ])
            .children(&mut [
                html!("div", {
                    .class([
                        "divide-y",
                        "divide-gray-200",
                        "dark:divide-gray-900",
                        "truncate"
                    ])
                    .children_signal_vec(histories.entries.signal_vec_cloned().map(|entry| {
                        link!(Route::Chapter(entry.chapter_id, entry.last_page_read).url(), {
                            .class([
                                "flex",
                                "p-2",
                                "m-2"
                            ])
                            .children(&mut [
                                html!("div", {
                                    .class([
                                        "pb-7/6",
                                        "mr-2",
                                        "flex-shrink-0"
                                    ])
                                    .children(&mut [
                                        html!("img", {
                                            .class([
                                                "w-16",
                                                "rounded",
                                                "object-cover"
                                            ])
                                            .attribute("src", &proxied_image_url(&entry.cover_url))
                                            .attribute("loading", "lazy")
                                        })
                                    ])
                                }),
                                html!("div", {
                                    .class(["flex-col"])
                                    .children(&mut [
                                        html!("h1", {
                                            .class([
                                                "text-gray-900",
                                                "dark:text-gray-50",
                                            ])
                                            .text(&entry.manga_title)
                                        }),
                                        html!("h2", {
                                            .class([
                                                "text-gray-900",
                                                "dark:text-gray-50",
                                            ])
                                            .text(&entry.chapter_title)
                                        }),
                                        html!("h2", {
                                            .class([
                                                "text-gray-900",
                                                "dark:text-gray-50",
                                            ])
                                            .text(&Self::calculate_days(entry.read_at))
                                        })
                                    ])
                                })
                            ])
                        })
                    }))
                }),
                html!("div", {
                    .child_signal(histories.spinner.signal().map(clone!(histories => move |x| if x {
                        Some(Spinner::render(&histories.spinner))
                    } else {
                        Some(html!("button", {
                            .class([
                                "w-full",
                                "text-gray-900",
                                "dark:text-gray-50",
                                "focus:outline-none"
                            ])
                            .class_signal("disabled", histories.is_entries_empty.signal())
                            .text_signal(histories.is_entries_empty.signal().map(|x|
                                if x {
                                    "No recent histories, favorite manga to see recent histories"
                                } else {
                                    "Load More"
                                }
                            ))
                            .event(clone!(histories => move |_: events::Click| {
                                Self::fetch_read_histories(histories.clone());
                            }))
                        }))
                    })))
                })
            ])
        })
    }

    fn calculate_days(at: chrono::NaiveDateTime) -> String {
        let timestamp = js_sys::Date::now();
        let secs: i64 = (timestamp / 1000.0).floor() as i64;
        let nanoes: u32 = (timestamp as u32 % 1000) * 1_000_000;
        let today = chrono::NaiveDateTime::from_timestamp(secs, nanoes);
        let days = today.date().signed_duration_since(at.date()).num_days();

        if days == 0 {
            "Today".to_string()
        } else if days == 1 {
            "Yesterday".to_string()
        } else if days > 1 && days <= 7 {
            format!("{} Days Ago", days)
        } else if days > 7 && days < 31 {
            format!("{} Weeks Ago", days / 7)
        } else {
            format!("{} Months Ago", days / 30)
        }
    }

    pub fn render(histories: Rc<Self>, _app: Rc<App>) -> Dom {
        Self::fetch_read_histories(histories.clone());
        html! {"div", {
            .class([
                "main",
            ])
            .children(&mut [
                Self::render_topbar(),
                Self::render_main(histories.clone())
            ])
        }}
    }
}
