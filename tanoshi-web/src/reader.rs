use crate::common::{events, ReaderSettings};
use crate::utils::{document, proxied_image_url, window, AsyncLoader};
use crate::{
    common::{Background, Direction, DisplayMode, ReaderMode},
    query::{fetch_chapter, update_page_read_at},
    utils::history,
};
use dominator::{clone, html, routing, svg, Dom, with_node};
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::HtmlImageElement;

enum Nav {
    None,
    Prev,
    Next
}

pub struct Reader {
    chapter_id: Mutable<i64>,
    manga_id: Mutable<i64>,
    manga_title: Mutable<String>,
    chapter_title: Mutable<String>,
    next_chapter: Mutable<Option<i64>>,
    prev_chapter: Mutable<Option<i64>>,
    prev_page: Mutable<Option<usize>>,
    current_page: Mutable<usize>,
    next_page: Mutable<Option<usize>>,
    pages: MutableVec<String>,
    pages_len: Mutable<usize>,
    reader_settings: Rc<ReaderSettings>,
    is_bar_visible: Mutable<bool>,
    loader: AsyncLoader,
}

impl Reader {
    pub fn new(chapter_id: i64, page: i64) -> Rc<Self> {
        Rc::new(Self {
            chapter_id: Mutable::new(chapter_id),
            manga_id: Mutable::new(0),
            manga_title: Mutable::new("".to_string()),
            chapter_title: Mutable::new("".to_string()),
            next_chapter: Mutable::new(None),
            prev_chapter: Mutable::new(None),
            prev_page: Mutable::new(None),
            current_page: Mutable::new(page as usize),
            next_page: Mutable::new(None),
            pages: MutableVec::new(),
            pages_len: Mutable::new(0),
            reader_settings: ReaderSettings::new(false, true),
            is_bar_visible: Mutable::new(true),
            loader: AsyncLoader::new(),
        })
    }

    fn fetch_detail(reader: Rc<Self>, chapter_id: i64, nav: Nav) {
        reader.loader.load(clone!(reader => async move {
            match fetch_chapter(chapter_id).await {
                Ok(result) => {
                    reader.manga_id.set_neq(result.manga.id);
                    reader.manga_title.set_neq(result.manga.title);
                    reader.chapter_title.set_neq(result.title);
                    reader.next_chapter.set_neq(result.next);
                    reader.prev_chapter.set_neq(result.prev);
                    reader.pages_len.set_neq(result.pages.len());
                    reader.pages.lock_mut().replace_cloned(result.pages.iter().map(|x| x.clone()).collect());
                    
                    reader.reader_settings.load_manga_reader_setting(result.manga.id);

                    match nav {
                        Nav::None => {},
                        Nav::Prev => {
                            let len = result.pages.len();
                            let page = match reader.reader_settings.reader_mode.get() {
                                ReaderMode::Continous => len - 1,
                                ReaderMode::Paged => {
                                    match reader.reader_settings.display_mode.get() {
                                        DisplayMode::Single => len - 1,
                                        DisplayMode::Double => {
                                            if len % 2 == 0 {
                                                len - 2
                                            } else {
                                                len - 1
                                            }
                                        }
                                    }
                                }
                            };

                            reader.current_page.set(page);
                        },
                        Nav::Next => {
                            reader.current_page.set(0);
                        },
                    }

                    Self::replace_state_with_url(reader);
                },
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }));
    }

    fn replace_state_with_url(reader: Rc<Self>) {
        history()
        .replace_state_with_url(
            &JsValue::null(), 
            "", 
            Some(
                format!(
                    "/chapter/{}/{}", 
                    reader.chapter_id.get(), 
                    reader.current_page.get() + 1
                ).as_str()
            )
        ).unwrap_throw();
    }

    fn update_page_read(reader: Rc<Self>, page: usize) {
        let chapter_id = reader.chapter_id.get();
        reader.loader.load(async move {
            match update_page_read_at(chapter_id, page as i64).await {
                Ok(_) => {}
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
                "animate__animated",
                "faster",
                "block",
                "fixed",
                "inset-x-0",
                "top-0",
                "z-50",
                "bg-gray-800",
                "content-end",
                "opacity-75",
                "pt-safe-top",
                "pb-2",
                "text-gray-50"
            ])
            .class_signal("animate__slideInDown", reader.is_bar_visible.signal())
            .class_signal("animate__slideOutUp", reader.is_bar_visible.signal().map(|x| !x))
            .children(&mut [
                html!("button", {
                    .class([
                        "mx-2",
                        "focus:outline-none"
                    ])
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
                        if history().length().unwrap_throw() == 0 {
                            routing::go_to_url("/");
                        } else {
                            history().back().unwrap_throw();
                        }
                    })
                }),
                html!("div", {
                    .class([
                        "flex",
                        "flex-col",
                        "text-center",
                        "truncate"
                    ])
                    .children(&mut [
                        html!("span", {
                            .text_signal(reader.chapter_title.signal_cloned().map(|t| t))
                        }),
                    ])
                }),
                html!("button", {
                    .class([
                        "mx-2",
                        "focus:outline-none"
                    ])
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
                "animate__animated",
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
            .class_signal("animate__slideInUp", reader.is_bar_visible.signal())
            .class_signal("animate__slideOutDown", reader.is_bar_visible.signal().map(|x| !x))
            .child_signal(reader.prev_chapter.signal().map(clone!(reader => move |prev_chapter| {
                match prev_chapter {
                    Some(prev) => Some(html!("button", {
                        .class("focus:outline-none")
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
                                        .attribute("d", "M11 17l-5-5m0 0l5-5m-5 5h12")
                                    })
                                ])
                            })
                        ])
                        .event(clone!(reader => move |_: events::Click| {
                            reader.chapter_id.set_neq(prev);
                        }))
                    })),
                    None => Some(html!("div",{}))
                }
            })))
            .children(&mut [
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
            ])
            .child_signal(reader.next_chapter.signal().map(clone!(reader => move |next_chapter| {
                match next_chapter {
                    Some(next) => Some(html!("button", {
                        .class("focus:outline-none")
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
                                        .attribute("d", "M13 7l5 5m0 0l-5 5m5-5H6")
                                    })
                                ])
                            })
                        ])
                        .event(clone!(reader => move |_: events::Click| {
                           reader.chapter_id.set(next);
                        }))
                    })),
                    None => Some(html!("div",{}))
                }
            })))
        })
    }

    pub fn render_vertical(reader: Rc<Self>) -> Dom {
        html!("div", {
            .children_signal_vec(reader.pages.signal_vec_cloned().enumerate().map(clone!(reader => move |(index, page)|
                html!("img", {
                    .class([
                        "page",
                        "mx-auto",
                    ])
                    .attribute("id", index.get().unwrap().to_string().as_str())
                    .attribute("src", &proxied_image_url(&page))
                    .class_signal("max-w-none", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::Height => true,
                        _ => false,
                    }))
                    .class_signal("object-contain", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::All => true,
                        _ => false,
                    }))
                    .class_signal("w-screen", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::Height => false,
                        _ => true
                    }))
                    .class_signal("h-screen", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::Width => false,
                        _ => true
                    }))
                    .event(|_: events::Error| {
                        log::error!("error loading image");
                    })
                    .event(clone!(reader => move |_: events::Click| {
                        reader.is_bar_visible.set_neq(!reader.is_bar_visible.get());
                    }))
                })
            )))
            .global_event(clone!(reader => move |_: events::Scroll| {
                let mut page_no = 0;
                let body_top = window().scroll_y().unwrap_throw();
                let page_collection = document().get_elements_by_class_name("page");
                for i in 0..page_collection.length() {
                    let page_top = page_collection.item(i).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap().offset_top() as f64;
                    if body_top > page_top {
                        page_no = i;
                        break;
                    }
                }
                reader.current_page.set_neq(page_no as usize);
            }))
        })
    }

    pub fn render_single(reader: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "flex",
                "items-center",
            ])
            .children_signal_vec(reader.pages.signal_vec_cloned().enumerate().map(clone!(reader => move |(index, page)|
                html!("img", {
                    .class([
                        "mx-auto",
                    ])
                    .class_signal("max-w-none", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::Height => true,
                        _ => false,
                    }))
                    .class_signal("object-contain", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::All => true,
                        _ => false,
                    }))
                    .class_signal("w-screen", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::Height => false,
                        _ => true
                    }))
                    .class_signal("h-screen", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::Width => false,
                        _ => true
                    }))
                    .visible_signal(reader.current_page.signal_cloned().map(clone!(reader => move |x| {
                        reader.prev_page.set_neq(x.checked_sub(1));
                        if x + 1 < reader.pages_len.get() {
                            reader.next_page.set_neq(Some(x + 1));
                        }

                        x == index.get().unwrap_or(0)
                    })))
                    .attribute("src", &proxied_image_url(&page))
                    .event(|_: events::Error| {
                        log::error!("error loading image");
                    })
                })
            )))
        })
    }

    pub fn render_double(reader: Rc<Self>) -> Dom {
        html!("div", {
            .attribute("id", "page-list")
            .class([
                "w-screen",
                "h-screen",
                "flex",
                "items-center"
            ])
            .class_signal("flex-row-reverse", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                Direction::LeftToRight => false,
                Direction::RightToLeft => true,
            }))
            .children_signal_vec(reader.pages.signal_vec_cloned().enumerate().map(clone!(reader => move |(index, page)|
                html!("img" => HtmlImageElement, {
                    .class([
                        "mx-auto"
                    ])
                    .attribute("id", format!("page-{}", index.get().unwrap_or(0)).as_str())
                    .class_signal("max-w-none", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::Height => true,
                        _ => false,
                    }))
                    .class_signal("object-contain", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::All => true,
                        _ => false,
                    }))
                    .class_signal("h-full", reader.reader_settings.fit.signal().map(|x| match x {
                        crate::common::Fit::Width => false,
                        _ => true
                    }))
                    .attribute("src", &proxied_image_url(&page))
                    .event(|_: events::Error| {
                        log::error!("error loading image");
                    })
                    .with_node!(img => {
                        .class_signal("w-1/2", reader.current_page.signal_cloned().map(clone!(reader, index, img => move |current_page| {
                            if index.get().unwrap() == current_page && img.natural_width() > img.natural_height
                            () {
                                false
                            } else {
                                match reader.reader_settings.fit.get() {
                                    crate::common::Fit::Height => false,
                                    _ => true
                                }
                            }
                        })))
                        .class_signal("hidden", reader.current_page.signal_cloned().map(clone!(reader, index, img => move |current_page| {
                            let mut hidden = true;
                            if index.get().unwrap() == current_page {
                                hidden = false;
                                if current_page > 0 {
                                    let prev_img = document().get_element_by_id(format!("page-{}", current_page - 1).as_str()).unwrap().dyn_into::<web_sys::HtmlImageElement>().unwrap();
                                    let sub = if prev_img.natural_width() > prev_img.natural_height() || current_page == 1 {
                                        1
                                    } else {
                                        2
                                    };
                                    reader.prev_page.set_neq(current_page.checked_sub(sub));
                                }
                            } else if index.get().unwrap() == current_page + 1 {
                                let prev_img = document().get_element_by_id(format!("page-{}", current_page).as_str()).unwrap().dyn_into::<web_sys::HtmlImageElement>().unwrap();
                                if img.natural_width() < img.natural_height() && prev_img.natural_width() < prev_img.natural_height() {
                                    hidden = false;
                                    if current_page + 2 < reader.pages_len.get() {
                                        reader.next_page.set_neq(Some(current_page + 2));
                                    } else {
                                        reader.next_page.set_neq(None);
                                    }
                                } else {
                                    if current_page + 1 < reader.pages_len.get() {
                                        reader.next_page.set_neq(Some(current_page + 1));
                                    } else {
                                        reader.next_page.set_neq(None);
                                    }
                                }
                            }
                            
                            hidden
                        })))
                        .event(clone!(reader, index => move |_: events::Load| {
                            let current_page = reader.current_page.get();
                            if index.get().unwrap() == current_page || index.get().unwrap() == current_page + 1 {
                                reader.current_page.replace(current_page);
                            }
                        }))
                    })
                })
            )))
        })
    }

    fn render_navigation(reader: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "fixed",
                "flex",
                "w-screen",
                "h-screen",
                "z-10",
                "cursor-pointer",
            ])
            .class_signal("flex-row-reverse", reader.reader_settings.direction.signal_cloned().map(|x| match x {
                Direction::LeftToRight => true,
                Direction::RightToLeft => false,
            }))
            .children(&mut [
                html!("div", {
                    .class([
                        "h-full",
                        "w-1/3",
                    ])
                    .attribute("id", "next")
                    .event(clone!(reader => move |_: events::Click| {
                        if let Some(next_page) = reader.next_page.get() {
                            reader.current_page.set_neq(next_page);
                        }
                    }))
                }),
                html!("div", {
                    .class([
                        "h-full",
                        "w-1/3",
                    ])
                    .attribute("id", "hide-bar")
                    .event(clone!(reader => move |_: events::Click| {
                        reader.is_bar_visible.set_neq(!reader.is_bar_visible.get());
                    }))
                }),
                html!("div", {
                    .class([
                        "h-full",
                        "w-1/3",
                    ])
                    .attribute("id", "prev")
                    .event(clone!(reader => move |_: events::Click| {
                        if let Some(prev_page) = reader.prev_page.get() {
                            reader.current_page.set_neq(prev_page);
                        }
                    }))
                })
            ])
        })
    }

    pub fn render(reader: Rc<Self>) -> Dom {
        // Self::fetch_detail(reader.clone(), reader.chapter_id.get());
        html!("div", {
            .future(reader.current_page.signal().for_each(clone!(reader => move |page| {
                Self::update_page_read(reader.clone(), page);
                Self::replace_state_with_url(reader.clone());
                async {}
            })))
            .future(reader.chapter_id.signal().for_each(clone!(reader => move |chapter_id| {
                let nav = match chapter_id {
                    _ if Some(chapter_id) == reader.prev_chapter.get() => Nav::Prev,
                    _ if Some(chapter_id) == reader.next_chapter.get() => Nav::Next,
                    _ => Nav::None,
                };

                Self::fetch_detail(reader.clone(), chapter_id, nav);

                async {}
            })))
            .class_signal("bg-gray-50", reader.reader_settings.background.signal_cloned().map(|x| match x {
                Background::White => true,
                Background::Black => false,
            }))
            .class_signal("bg-gray-900", reader.reader_settings.background.signal_cloned().map(|x| match x {
                Background::White => false,
                Background::Black => true,
            }))
            .children(&mut [
                Self::render_topbar(reader.clone()),
            ])
            .child_signal(reader.reader_settings.reader_mode.signal_cloned().map(clone!(reader => move |x| match x {
                ReaderMode::Continous => Some(Self::render_vertical(reader.clone())),
                ReaderMode::Paged => Some(html!("div", {
                    .children(&mut [
                        Self::render_navigation(reader.clone())
                    ])
                    .child_signal(reader.reader_settings.display_mode.signal_cloned().map(clone!(reader => move |x| match x {
                        DisplayMode::Single => Some(Self::render_single(reader.clone())),
                        DisplayMode::Double => Some(Self::render_double(reader.clone())),
                    })))
                }))
            })))
            .children(&mut [
                Self::render_bottombar(reader.clone()),
                ReaderSettings::render(reader.reader_settings.clone()),
            ])
        })
    }
}
