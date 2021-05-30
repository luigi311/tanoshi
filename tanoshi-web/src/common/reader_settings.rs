use dominator::{clone, events, html, Dom};
use futures_signals::signal::{Mutable, Signal, SignalExt};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use web_sys::console::info;

use crate::{manga, utils::local_storage};

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum ReaderMode {
    Continous,
    Paged,
}

impl Default for ReaderMode {
    fn default() -> Self {
        ReaderMode::Paged
    }
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum DisplayMode {
    Single,
    Double,
}

impl Default for DisplayMode {
    fn default() -> Self {
        DisplayMode::Single
    }
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::LeftToRight
    }
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum Background {
    White,
    Black,
}

impl Default for Background {
    fn default() -> Self {
        Background::White
    }
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum Fit {
    Height,
    Width,
    All
}

impl Default for Fit {
    fn default() -> Self {
        Fit::All
    }
}

#[derive(PartialEq, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub reader_mode: ReaderMode,
    pub display_mode: DisplayMode,
    pub direction: Direction,
    pub background: Background,
    pub fit: Fit,
}

#[derive(PartialEq, Clone)]
pub struct Page {
    id: i64,
    url: String,
}

const MODAL_CLASS: [&'static str; 20] = [
    "fixed",
    "shadow",
    "w-full",
    "xl:w-1/3",
    "px-2",
    "pt-2",
    "rounded-t",
    "xl:rounded",
    "mb-0",
    "xl:mb-safe-bottom",
    "mx-auto",
    "inset-x-0",
    "bottom-0",
    "bg-white",
    "dark:bg-gray-800",
    "z-50",
    "text-gray-900",
    "dark:text-gray-50",
    "animate__animated",
    "pb-safe-bottom"
];

const NON_MODAL_CLASS: [&'static str; 10] = [
    "w-full",
    "p-2",
    "rounded",
    "mx-auto",
    "bg-white",
    "dark:bg-gray-900",
    "shadow",
    "dark:shadow-none",
    "text-gray-900",
    "dark:text-gray-50",
];

pub struct ReaderSettings {
    use_modal: Mutable<bool>,
    first_render: Mutable<bool>,
    show: Mutable<bool>,
    manga_id: Mutable<i64>,
    pub reader_mode: Mutable<ReaderMode>,
    pub display_mode: Mutable<DisplayMode>,
    pub direction: Mutable<Direction>,
    pub background: Mutable<Background>,
    pub fit: Mutable<Fit>,
}

impl ReaderSettings {
    pub fn new(show: bool, use_modal: bool) -> Rc<Self> {
        let settings = if let Ok(settings) = local_storage().get_item("settings:reader") {
            if let Some(settings) = settings {
                serde_json::from_str::<Settings>(&settings).unwrap_or_default()
            } else {
                Settings::default()
            }
        } else {
            Settings::default()
        };

        Rc::new(ReaderSettings {
            use_modal: Mutable::new(use_modal),
            first_render: Mutable::new(if use_modal { true } else { false }),
            show: Mutable::new(show),
            manga_id: Mutable::new(0),
            reader_mode: Mutable::new(settings.reader_mode),
            display_mode: Mutable::new(settings.display_mode),
            direction: Mutable::new(settings.direction),
            background: Mutable::new(settings.background),
            fit: Mutable::new(settings.fit)
        })
    }

    pub fn load_manga_reader_setting(&self, manga_id: i64) {
        if manga_id == 0 {
            return;
        }

        self.manga_id.replace(manga_id);

        let key = ["settings:reader", &manga_id.to_string()].join(":");
        let settings = if let Ok(settings) = local_storage().get_item(&key) {
            if let Some(settings) = settings {
                serde_json::from_str::<Settings>(&settings).unwrap_or_default()
            } else {
                return;
            }
        } else {
            return;
        };

        self.reader_mode.replace(settings.reader_mode);
        self.display_mode.replace(settings.display_mode);
        self.direction.replace(settings.direction);
        self.background.replace(settings.background);
    }

    pub fn toggle_show(&self) {
        self.show.replace_with(|show| !*show);
        self.first_render.replace_with(|_| false);
    }

    pub fn render_apply_button(reader: Rc<Self>) -> Dom {
        html!("button", {
            .text("Apply")
            .event(clone!(reader => move |_: events::Click| {
                let settings = Settings {
                    reader_mode: reader.reader_mode.get(),
                    display_mode: reader.display_mode.get(),
                    direction: reader.direction.get(),
                    background: reader.background.get(),
                    fit: reader.fit.get(),
                };

                let mut key = "settings:reader".to_string();
                if *reader.manga_id.lock_ref() > 0 {
                    key = [key, (*reader.manga_id.lock_ref()).to_string()].join(":");
                }

                let _ = local_storage().set_item(&key, &serde_json::to_string(&settings).unwrap());
                if *reader.use_modal.lock_ref() {
                    reader.show.set_neq(false);
                }
            }))
        })
    }

    fn render_reader_mode(reader: Rc<Self>) -> Dom {
        html!("div", {
            .class([
                "w-full"
            ])
            .children(&mut [
                html!("div", {
                    .class([
                        "w-full",
                        "flex",
                        "justify-between",
                        "mb-2"
                    ])
                    .class_signal([
                        "border-b",
                        "border-gray-100",
                        "dark:border-gray-800"
                    ], reader.use_modal.signal())
                    .visible_signal(reader.use_modal.signal())
                    .children(&mut [
                        html!("h1", {
                            .text("Settings")
                        }),
                        Self::render_apply_button(reader.clone())
                    ])
                }),
                html!("label", {
                    .text("Reader Mode")
                }),
                html!("div", {
                    .class([
                        "w-full",
                        "bg-gray-200",
                        "dark:bg-gray-700",
                        "rounded",
                        "p-1"
                    ])
                    .children(&mut [
                        html!("button", {
                            .class("w-1/2")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.reader_mode.signal_cloned().map(|x| match x {
                                ReaderMode::Continous => true,
                                ReaderMode::Paged => false,
                            }))
                            .text("Continous")
                            .event(clone!(reader => move |_: events::Click| reader.reader_mode.set_neq(ReaderMode::Continous)))
                        }),
                        html!("button", {
                            .class("w-1/2")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.reader_mode.signal_cloned().map(|x| match x {
                                ReaderMode::Continous => false,
                                ReaderMode::Paged => true,
                            }))
                            .text("Paged")
                            .event(clone!(reader => move |_: events::Click| reader.reader_mode.set_neq(ReaderMode::Paged)))
                        }),
                    ])
                })
            ])
        })
    }

    fn render_display_mode(reader: Rc<Self>) -> Dom {
        html!("div", {
            .class_signal("opacity-50", reader.reader_mode.signal_cloned().map(|x| match x {
                ReaderMode::Continous => true,
                ReaderMode::Paged => false,
            }))
            .children(&mut [
                html!("label", {
                    .class("w-full")
                    .text("Display Mode")
                }),
                html!("div", {
                    .class([
                        "w-full",
                        "bg-gray-200",
                        "dark:bg-gray-700",
                        "rounded",
                        "p-1"
                    ])
                    .children(&mut [
                        html!("button", {
                            .class("w-1/2")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.display_mode.signal_cloned().map(|x| match x {
                                DisplayMode::Single => true,
                                DisplayMode::Double => false,
                            }))
                            .attribute_signal("disabled", reader.reader_mode.signal_cloned().map(|x| match x {
                                ReaderMode::Continous => Some("true"),
                                ReaderMode::Paged => None,
                            }))
                            .text("Single")
                            .event(clone!(reader => move |_: events::Click| reader.display_mode.set_neq(DisplayMode::Single)))
                        }),
                        html!("button", {
                            .class("w-1/2")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.display_mode.signal_cloned().map(|x| match x {
                                DisplayMode::Single => false,
                                DisplayMode::Double => true,
                            }))
                            .attribute_signal("disabled", reader.reader_mode.signal_cloned().map(|x| match x {
                                ReaderMode::Continous => Some("true"),
                                ReaderMode::Paged => None,
                            }))
                            .text("Double")
                            .event(clone!(reader => move |_: events::Click| reader.display_mode.set_neq(DisplayMode::Double)))
                        }),
                    ])
                })
            ])
        })
    }

    fn render_direction(reader: Rc<Self>) -> Dom {
        html!("div", {
            .class_signal("opacity-50", reader.reader_mode.signal_cloned().map(|x| match x {
                ReaderMode::Continous => true,
                ReaderMode::Paged => false,
            }))
            .children(&mut [
                html!("label", {
                    .text("Direction")
                }),
                html!("div", {
                    .class([
                        "w-full",
                        "bg-gray-200",
                        "dark:bg-gray-700",
                        "rounded",
                        "p-1"
                    ])
                    .children(&mut [
                        html!("button", {
                            .class("w-1/2")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.direction.signal_cloned().map(|x| match x {
                                Direction::LeftToRight => true,
                                Direction::RightToLeft => false,
                            }))
                            .attribute_signal("disabled", reader.reader_mode.signal_cloned().map(|x| match x {
                                ReaderMode::Continous => Some("true"),
                                ReaderMode::Paged => None,
                            }))
                            .text("Left to Right")
                            .event(clone!(reader => move |_: events::Click| reader.direction.set_neq(Direction::LeftToRight)))
                        }),
                        html!("button", {
                            .class("w-1/2")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.direction.signal_cloned().map(|x| match x {
                                Direction::LeftToRight => false,
                                Direction::RightToLeft => true,
                            }))
                            .attribute_signal("disabled", reader.reader_mode.signal_cloned().map(|x| match x {
                                ReaderMode::Continous => Some("true"),
                                ReaderMode::Paged => None,
                            }))
                            .text("Right to Left")
                            .event(clone!(reader => move |_: events::Click| reader.direction.set_neq(Direction::RightToLeft)))
                        }),
                    ])
                })
            ])
        })
    }

    fn render_background(reader: Rc<Self>) -> Dom {
        html!("div", {
            .children(&mut [
                html!("label", {
                    .text("Background")
                }),
                html!("div", {
                    .class([
                        "w-full",
                        "bg-gray-200",
                        "dark:bg-gray-700",
                        "rounded",
                        "p-1"
                    ])
                    .children(&mut [
                        html!("button", {
                            .class("w-1/2")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.background.signal_cloned().map(|x| match x {
                                Background::Black => true,
                                Background::White => false,
                            }))
                            .text("Black")
                            .event(clone!(reader => move |_: events::Click| reader.background.set_neq(Background::Black)))
                        }),
                        html!("button", {
                            .class("w-1/2")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.background.signal_cloned().map(|x| match x {
                                Background::Black => false,
                                Background::White => true,
                            }))
                            .text("White")
                            .event(clone!(reader => move |_: events::Click| reader.background.set_neq(Background::White)))
                        }),
                    ])
                })
            ])
        })
    }

    fn render_fit_screen(reader: Rc<Self>) -> Dom {
        html!("div", {
            .children(&mut [
                html!("label", {
                    .text("Fit")
                }),
                html!("div", {
                    .class([
                        "w-full",
                        "bg-gray-200",
                        "dark:bg-gray-700",
                        "rounded",
                        "p-1"
                    ])
                    .children(&mut [
                        html!("button", {
                            .class("w-1/3")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.fit.signal_cloned().map(|x| match x {
                                Fit::Height => true,
                                Fit::Width => false,
                                Fit::All => false
                            }))
                            .text("Height")
                            .event(clone!(reader => move |_: events::Click| reader.fit.set_neq(Fit::Height)))
                        }),
                        html!("button", {
                            .class("w-1/3")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.fit.signal_cloned().map(|x| match x {
                                Fit::Height => false,
                                Fit::Width => true,
                                Fit::All => false
                            }))
                            .text("Width")
                            .event(clone!(reader => move |_: events::Click| reader.fit.set_neq(Fit::Width)))
                        }),
                        html!("button", {
                            .class("w-1/3")
                            .class_signal(["bg-gray-50", "dark:bg-gray-600", "rounded", "shadow"], reader.fit.signal_cloned().map(|x| match x {
                                Fit::Height => false,
                                Fit::Width => false,
                                Fit::All => true
                            }))
                            .text("All")
                            .event(clone!(reader => move |_: events::Click| reader.fit.set_neq(Fit::All)))
                        }),
                    ])
                })
            ])
        })
    }

    pub fn render(reader: Rc<Self>) -> Dom {
        html!("div", {
            .class_signal(MODAL_CLASS, reader.use_modal.signal())
            .class_signal(NON_MODAL_CLASS, reader.use_modal.signal().map(|x| !x))
            .class_signal("animate__slideInUp", reader.show.signal())
            .class_signal("animate__slideOutDown", reader.show.signal().map(|x| !x))
            .visible_signal(reader.first_render.signal().map(|x| !x))
            .children(&mut [
                html!("div", {
                    .children(&mut [
                        Self::render_reader_mode(reader.clone()),
                        Self::render_display_mode(reader.clone()),
                        Self::render_direction(reader.clone()),
                        Self::render_background(reader.clone()),
                        Self::render_fit_screen(reader.clone()),
                    ])
                })
            ])
        })
    }
}
