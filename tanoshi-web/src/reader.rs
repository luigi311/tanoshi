use crate::{common::{Background, Direction, DisplayMode, ReaderMode}, query::{fetch_chapter, update_page_read_at}};
use crate::app::App;
use crate::common::{events, Spinner, Route, ReaderSettings};
use crate::utils::{proxied_image_url};
use dominator::{html, svg, Dom, clone, link, routing};
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use std::rc::Rc;
use web_sys::window;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use serde::{Serialize, Deserialize};

#[derive(PartialEq, Clone)]
pub struct Page {
    id: i64,
    url: String,
}

pub struct Reader {
    chapter_id: i64,
    manga_id: Mutable<i64>,
    manga_title: Mutable<String>,
    chapter_title: Mutable<String>,
    next_chapter: Mutable<Option<i64>>,
    prev_chapter: Mutable<Option<i64>>,
    current_page: Mutable<usize>,
    pages: MutableVec<Page>,
    pages_len: Mutable<usize>,
    reader_settings: Rc<ReaderSettings>,
    is_bar_visible: Mutable<bool>,
}

impl Reader {
    pub fn new(chapter_id: i64) -> Rc<Self> {
        Rc::new(Self {
            chapter_id,
            manga_id: Mutable::new(0),
            manga_title: Mutable::new("".to_string()),
            chapter_title: Mutable::new("".to_string()),
            next_chapter: Mutable::new(None),
            prev_chapter: Mutable::new(None),
            current_page: Mutable::new(0),
            pages: MutableVec::new(),
            pages_len: Mutable::new(0),
            reader_settings: ReaderSettings::new(false, true),
            is_bar_visible: Mutable::new(true),
        })
    }

    pub fn fetch_detail(reader: Rc<Self>, app: Rc<App>) {
        let spinner = app.spinner.clone();
        app.spinner.set_active(true);
        app.spinner.set_fullscreen(true);
        app.loader.load(clone!(reader => async move {
            match fetch_chapter(reader.chapter_id).await {
                Ok(result) => {
                    reader.manga_id.set(result.manga.id);
                    reader.manga_title.set(result.manga.title);
                    reader.chapter_title.set(result.title);
                    reader.next_chapter.set(result.next);
                    reader.prev_chapter.set(result.prev);
                    reader.pages.lock_mut().replace_cloned(result.pages.iter().map(|page| Page{id: page.id, url: page.url.clone()}).collect());
                    reader.pages_len.set_neq(result.pages.len());


                    reader.reader_settings.load_manga_reader_setting(result.manga.id);
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }

            spinner.set_active(false);
        }));
    }

    pub fn update_page_read(app: Rc<App>, page_id: i64) {
        app.loader.load(async move {
            match update_page_read_at(page_id).await {
                Ok(_) => {},
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        });
    }

    pub fn render_topbar(reader: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "flex",
                "justify-between",
                "items-center",
                "animated",
                "slideInDown",
                "faster",
                "block",
                "fixed",
                "inset-x-0",
                "top-0",
                "z-40",
                "bg-gray-800",
                "content-end",
                "opacity-75",
                "pt-safe-top",
                "pb-2",
                "text-gray-50"
            ])
            .visible_signal(reader.is_bar_visible.signal_cloned())
            .children(&mut [
                html!("button", {
                    .class("mx-2")
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute("fill", "none")
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .class(["w-6", "h-6"])
                            .children(&mut [
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "2")
                                    .attribute("d", "M15 19l-7-7 7-7")
                                })
                            ])
                        })
                    ])
                    .event(|_: events::Click| {
                        let history = window().unwrap_throw().history().unwrap_throw();
                        if history.length().unwrap_throw() > 1 {
                            history.back().unwrap_throw();
                        } else {
                            routing::go_to_url("/");
                        }
                    })
                }),
                html!("div", {
                    .class([
                        "flex", 
                        "flex-col",
                        "text-center"
                    ])
                    .children(&mut [
                        html!("span", {
                            .class("truncate")
                            .text_signal(reader.chapter_title.signal_cloned().map(|t| t))
                        }),
                    ])
                }),
                html!("button", {
                    .class("mx-2")
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .attribute("fill", "none")
                            .class("w-6")
                            .class("h-6")
                            .children(&mut [
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "1")
                                    .class("heroicon-ui")
                                    .attribute("d", "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z")
                                }),
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "1")
                                    .class("heroicon-ui")
                                    .attribute("d", "M15 12a3 3 0 11-6 0 3 3 0 016 0z")
                                })
                            ])
                        })
                    ])
                    .event(clone!(reader => move |_: events::Click| {
                        reader.reader_settings.toggle_show();
                    }))
                })
            ])
        })
    }

    pub fn render_bottombar(reader: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "flex",
                "justify-between",
                "items-center",
                "animated",
                "slideInDown",
                "faster",
                "block",
                "fixed",
                "inset-x-0",
                "bottom-0",
                "z-40",
                "bg-gray-800",
                "content-end",
                "opacity-75",
                "pt-2",
                "pb-safe-bottom",
                "text-gray-50"
            ])
            .visible_signal(reader.is_bar_visible.signal_cloned())
            .children(&mut [
                html!("button", {
                    .child_signal(reader.prev_chapter.signal().map(clone!(reader => move |prev_chapter| {
                        let prev_url = match prev_chapter {
                            Some(prev) => Route::Chapter(prev).url(),
                            None => Route::Manga(reader.manga_id.get()).url()
                        };

                        Some(link!(prev_url, {
                            .children(&mut [
                                svg!("svg", {
                                    .attribute("xmlns", "http://www.w3.org/2000/svg")
                                    .attribute("fill", "none")
                                    .attribute("viewBox", "0 0 24 24")
                                    .attribute("stroke", "currentColor")
                                    .class(["w-6", "h-6", "ml-2"])
                                    .children(&mut [
                                        svg!("path", {
                                            .attribute("stroke-linecap", "round")
                                            .attribute("stroke-linejoin", "round")
                                            .attribute("stroke-width", "2")
                                            .attribute("d", "M11 19l-7-7 7-7m8 14l-7-7 7-7")
                                        })
                                    ])
                                })
                            ])
                        }))
                    })))
                }),
                html!("div", {
                    .children(&mut [
                        html!("span", {
                            .text_signal(reader.current_page.signal().map(|p| (p + 1).to_string()))
                        }),
                        html!("span", {
                            .text("/")
                        }),
                        html!("span", {
                            .text_signal(reader.pages_len.signal().map(|len| len.to_string()))
                        }),
                    ])
                }),
                html!("button", {
                    .child_signal(reader.next_chapter.signal().map(clone!(reader => move |next_chapter| {
                         let next_url = match next_chapter {
                            Some(next) => Route::Chapter(next).url(),
                            None => Route::Manga(reader.manga_id.get()).url()
                         };

                         Some(link!(next_url, {
                            .children(&mut [
                                svg!("svg", {
                                    .attribute("xmlns", "http://www.w3.org/2000/svg")
                                    .attribute("fill", "none")
                                    .attribute("viewBox", "0 0 24 24")
                                    .attribute("stroke", "currentColor")
                                    .class(["w-6", "h-6", "mr-2"])
                                    .children(&mut [
                                        svg!("path", {
                                            .attribute("stroke-linecap", "round")
                                            .attribute("stroke-linejoin", "round")
                                            .attribute("stroke-width", "2")
                                            .attribute("d", "M13 5l7 7-7 7M5 5l7 7-7 7")
                                        })
                                    ])
                                })
                            ])
                        }))
                    })))
                })
            ])
        })
    }

    pub fn render_vertical(reader: Rc<Self>, app: Rc<App>) -> Dom {
        html!("div", {
            .class("h-screen")
            .class("overflow-y-auto")
            .children_signal_vec(reader.pages.signal_vec_cloned().enumerate().map(clone!(reader => move |(index, page)|
                html!("img", {
                    .class("page")
                    .class("my-1")
                    .class("mx-auto")
                    .attribute("id", index.get().unwrap().to_string().as_str())
                    .attribute("src", &proxied_image_url(&page.url))
                    .event(|_: events::Error| {
                        log::error!("error loading image");
                    })
                    .event(clone!(reader => move |_: events::Click| {
                        reader.is_bar_visible.set_neq(!reader.is_bar_visible.get());
                    }))
                })
            )))
            .event_preventable(clone!(reader, app => move |event: events::Scroll| {
                event.prevent_default();
                let mut page_no = 0;
                let scroll_top = event.target().unwrap().dyn_into::<web_sys::HtmlElement>().unwrap().scroll_top();
                let page_collection = window().unwrap().document().unwrap().get_elements_by_class_name("page");
                for i in 0..page_collection.length() {
                    if scroll_top > page_collection.item(i).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap().offset_top() {
                        page_no = i;
                    }
                }
                reader.current_page.set_if(page_no as usize, |before, after| {
                    if *before != *after {
                        Self::update_page_read(app.clone(), reader.pages.lock_ref().get(*after).unwrap().id);
                        true
                    } else {
                        false
                    }
                });
            }))
        })
    }

    pub fn render_single(reader: Rc<Self>, app: Rc<App>) -> Dom {
        html!("div", {
            .children(&mut [
                html!("div", {
                    .class([
                        "h-screen",
                        "w-1/3",
                        "cursor-pointer",
                        "fixed"
                    ])
                    .class_signal("left-0", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => false,
                        Direction::RightToLeft => true,
                    }))
                    .class_signal("right-0", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => true,
                        Direction::RightToLeft => false,
                    }))
                    .event(clone!(reader, app => move |_: events::Click| {
                        reader.current_page.set_if(reader.current_page.get() + 1, |_, after| {
                            if *after < reader.pages.lock_ref().len()  {
                                Self::update_page_read(app.clone(), reader.pages.lock_ref().get(*after).unwrap().id);
                                true
                            } else {
                                false
                            }
                        });
                    }))
                }),
                html!("div", {
                    .class([
                        "h-screen",
                        "w-1/3",
                        "cursor-pointer",
                        "fixed",
                        "inset-x-0",
                        "mx-auto"
                    ])
                    .event(clone!(reader, app => move |_: events::Click| {
                        reader.is_bar_visible.set_neq(!reader.is_bar_visible.get());
                    }))
                }),
                html!("div", {
                    .class([
                        "h-screen",
                        "w-1/3",
                        "cursor-pointer",
                        "fixed"
                    ])
                    .class_signal("left-0", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => true,
                        Direction::RightToLeft => false,
                    }))
                    .class_signal("right-0", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => false,
                        Direction::RightToLeft => true,
                    }))
                    .event(clone!(reader, app => move |_: events::Click| {
                        reader.current_page.set_if(reader.current_page.get().checked_sub(1).unwrap_or(0), |before, after| {
                            if *before != *after  {
                                Self::update_page_read(app.clone(), reader.pages.lock_ref().get(*after).unwrap().id);
                                true
                            } else {
                                false
                            }
                        })
                    }))
                }),
                html!("div", {
                    .children_signal_vec(reader.pages.signal_vec_cloned().enumerate().map(clone!(reader => move |(index, page)|
                        html!("img", {
                            .class([
                                "mx-auto",
                                "h-screen"
                            ])
                            .visible_signal(reader.current_page.signal_cloned().map(move |x| x == index.get().unwrap_or(0)))
                            .attribute("src", &proxied_image_url(&page.url))
                            .event(|_: events::Error| {
                                log::error!("error loading image");
                            })
                        })
                    )))
                })
            ])
        })
    }

    pub fn render_double(reader: Rc<Self>, app: Rc<App>) -> Dom {
        html!("div", {
            .children(&mut [
                html!("div", {
                    .class([
                        "h-screen",
                        "w-1/3",
                        "cursor-pointer",
                        "fixed"
                    ])
                    .class_signal("left-0", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => false,
                        Direction::RightToLeft => true,
                    }))
                    .class_signal("right-0", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => true,
                        Direction::RightToLeft => false,
                    }))
                    .event(clone!(reader, app => move |_: events::Click| {
                        reader.current_page.set_if(reader.current_page.get() + 2, |before, after| {
                            if *after < reader.pages.lock_ref().len() {
                                Self::update_page_read(app.clone(), reader.pages.lock_ref().get(*before).unwrap().id);
                                true
                            } else {
                                false
                            }
                        });
                    }))
                }),
                html!("div", {
                    .class([
                        "h-screen",
                        "w-1/3",
                        "cursor-pointer",
                        "fixed",
                        "inset-x-0",
                        "mx-auto"
                    ])
                    .event(clone!(reader, app => move |_: events::Click| {
                        reader.is_bar_visible.set_neq(!reader.is_bar_visible.get());
                    }))
                }),
                html!("div", {
                    .class([
                        "h-screen",
                        "w-1/3",
                        "right-0",
                        "cursor-pointer",
                        "fixed"
                    ])
                    .class_signal("left-0", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => true,
                        Direction::RightToLeft => false,
                    }))
                    .class_signal("right-0", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => false,
                        Direction::RightToLeft => true,
                    }))
                    .event(clone!(reader, app => move |_: events::Click| {
                        reader.current_page.set_if(reader.current_page.get().checked_sub(2).unwrap_or(0), |before, after| {
                            if *before != *after {
                                Self::update_page_read(app.clone(), reader.pages.lock_ref().get(*before).unwrap().id);
                                true
                            } else {
                                false
                            }
                        });
                    }))
                }),
                html!("div", {
                    .class([
                        "flex",
                        "h-screen",
                        "justify-center",
                        "overflow-visible"
                    ])
                    .class_signal("flex-row-reverse", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => false,
                        Direction::RightToLeft => true,
                    }))
                    .class_signal("flex-row", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                        Direction::LeftToRight => true,
                        Direction::RightToLeft => false,
                    }))
                    .children_signal_vec(reader.pages.signal_vec_cloned().enumerate().map(clone!(reader => move |(index, page)|
                        html!("img", {
                            .class(["object-contain", "h-screen"])
                            .visible_signal(reader.current_page.signal_cloned().map(move |x| x == index.get().unwrap_or(0) || x + 1 == index.get().unwrap_or(0)))
                            .attribute("src", &proxied_image_url(&page.url))
                            .event(|_: events::Error| {
                                log::error!("error loading image");
                            })
                        })
                    )))
                })
            ])
        })
    }

    pub fn render(reader: Rc<Self>, app: Rc<App>) -> Dom {
        Self::fetch_detail(reader.clone(), app.clone());
        html!("div", {
            .class("h-screen")
            .children(&mut [
                Self::render_topbar(reader.clone()),
                html!("div", {
                    .class_signal("bg-gray-50", reader.reader_settings.background.signal_cloned().map(|x| match x {
                        Background::White => true,
                        Background::Black => false,
                    }))
                    .class_signal("bg-gray-900", reader.reader_settings.background.signal_cloned().map(|x| match x {
                        Background::White => false,
                        Background::Black => true,
                    }))
                    .child_signal(reader.reader_settings.reader_mode.signal_cloned().map(clone!(reader, app => move |x| match x {
                        ReaderMode::Continous => Some(Self::render_vertical(reader.clone(), app.clone())),
                        ReaderMode::Paged => Some(html!("div", {
                            .child_signal(reader.reader_settings.display_mode.signal_cloned().map(clone!(reader, app => move |x| match x {
                                DisplayMode::Single => Some(Self::render_single(reader.clone(), app.clone())),
                                DisplayMode::Double => Some(Self::render_double(reader.clone(), app.clone())),
                            })))
                        }))
                    })))
                }),
                Self::render_bottombar(reader.clone()),
                ReaderSettings::render(reader.reader_settings.clone()),
                Spinner::render(&app.spinner)
            ])
        })
    }
}
