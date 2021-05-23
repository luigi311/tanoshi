use std::rc::Rc;

use dominator::{html, link, svg, Dom};
use futures_signals::signal::SignalExt;
use super::{Route, SettingCategory};

const ACTIVE_CLASS: [&'static str; 6] = [
    "text-accent",
    "bg-transparent",
    "xl:text-gray-900",
    "xl:dark:text-gray-100",
    "xl:bg-gray-300",
    "xl:dark:bg-gray-700",
];

const DEACTIVE_CLASS: [&'static str; 4] = [
    "text-gray-900",
    "dark:text-gray-50",
    "xl:text-gray-900",
    "xl:dark:text-gray-50",
];

const LINK_CLASS: [&'static str; 6] = [
    "flex", 
    "flex-col",
    "lg:flex-row",
    "rounded", 
    "px-2",
    "items-center"
];

pub struct Bottombar {}

impl Bottombar {
    pub fn new() -> Rc<Self> {
        Rc::new(Bottombar {})
    }

    pub fn render() -> Dom {
        html!("div", {
            .class([
                "fixed",
                "inset-x-0",
                "bottom-0",
                "xl:inset-y-0",
                "xl:left-0",
                "w-full",
                "xl:w-48",
                "z-50",
                "flex",
                "xl:flex-col",
                "justify-evenly",
                "xl:justify-start",
                "border-t",
                "xl:border-r",
                "border-gray-200",
                "dark:border-gray-800",
                "safe-bottom",
                "bg-gray-100",
                "dark:bg-gray-900",
                "pt-2",
                "xl:pt-0",
                "px-2",
                "xl:px-3",
                "pb-safe-bottom",
                "xl:pt-safe-top",
            ])
            .children(&mut [
                link!(Route::Library.url(), {
                    .class(LINK_CLASS)
                    .class_signal(ACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Library => true,
                        _ => false,
                    }))
                    .class_signal(DEACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Library => false,
                        _ => true,
                    }))
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .attribute("fill", "none")
                            .class([
                                "w-6",
                                "h-6",
                                "my-0",
                                "xl:my-2",
                            ])
                            .children(&mut [
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "1")
                                    .class("heroicon-ui")
                                    .attribute("d", "M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z")
                                })
                            ])
                        }),
                        html!("span", {
                            .class([
                                "text-xs",
                                "xl:text-base",
                                "my-auto",
                                "mx-2"
                            ])
                            .text("Library")
                        })
                    ])
                }),
                link!(Route::Catalogue{id: 0, latest: false}.url(), {
                    .class(LINK_CLASS)
                    .class_signal(ACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Catalogue{id, latest} => true,
                        _ => false,
                    }))
                    .class_signal(DEACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Catalogue{id, latest} => false,
                        _ => true,
                    }))
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .attribute("fill", "none")
                            .class([
                                "w-6",
                                "h-6",
                                "my-0",
                                "xl:my-2",
                            ])
                            .children(&mut [
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "1")
                                    .class("heroicon-ui")
                                    .attribute("d", "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10")
                                })
                            ])
                        }),
                        html!("span", {
                            .class([
                                "text-xs",
                                "xl:text-base",
                                "my-auto",
                                "mx-2"
                            ])
                            .text("Catalogue")
                        })
                    ])
                }),
                link!(Route::Updates.url(), {
                    .class(LINK_CLASS)
                    .class_signal(ACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Updates => true,
                        _ => false,
                    }))
                    .class_signal(DEACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Updates => false,
                        _ => true,
                    }))
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .attribute("fill", "none")
                            .class([
                                "w-6",
                                "h-6",
                                "my-0",
                                "xl:my-2",
                            ])
                            .children(&mut [
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "1")
                                    .class("heroicon-ui")
                                    .attribute("d", "M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9")
                                })
                            ])
                        }),
                        html!("span", {
                            .class([
                                "text-xs",
                                "xl:text-base",
                                "my-auto",
                                "mx-2"
                            ])
                            .text("Updates")
                        })
                    ])
                }),
                link!(Route::Histories.url(), {
                    .class(LINK_CLASS)
                    .class_signal(ACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Histories => true,
                        _ => false,
                    }))
                    .class_signal(DEACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Histories => false,
                        _ => true,
                    }))
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .attribute("fill", "none")
                            .class([
                                "w-6",
                                "h-6",
                                "my-0",
                                "xl:my-2",
                            ])
                            .children(&mut [
                                svg!("path", {
                                    .attribute("stroke-linecap", "round")
                                    .attribute("stroke-linejoin", "round")
                                    .attribute("stroke-width", "1")
                                    .class("heroicon-ui")
                                    .attribute("d", "M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z")
                                })
                            ])
                        }),
                        html!("span", {
                            .class([
                                "text-xs",
                                "xl:text-base",
                                "my-auto",
                                "mx-2"
                            ])
                            .text("History")
                        })
                    ])
                }),
                link!(Route::Settings(SettingCategory::None).url(), {
                    .class(LINK_CLASS)
                    .class_signal(ACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Settings(_) => true,
                        _ => false,
                    }))
                    .class_signal(DEACTIVE_CLASS, Route::signal().map(|x| match x {
                        Route::Settings(_) => false,
                        _ => true,
                    }))
                    .children(&mut [
                        svg!("svg", {
                            .attribute("xmlns", "http://www.w3.org/2000/svg")
                            .attribute("viewBox", "0 0 24 24")
                            .attribute("stroke", "currentColor")
                            .attribute("fill", "none")
                            .class([
                                "w-6",
                                "h-6",
                                "my-0",
                                "xl:my-2",
                            ])
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
                        }),
                        html!("span", {
                            .class([
                                "text-xs",
                                "xl:text-base",
                                "my-auto",
                                "mx-2"
                            ])
                            .text("Settings")
                        })
                    ])
                }),
            ])
        })
    }
}
