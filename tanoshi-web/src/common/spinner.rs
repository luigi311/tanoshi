use dominator::{html, Dom};
use futures_signals::signal::Mutable;
use futures_signals::signal::{SignalExt, Signal};
use std::rc::Rc;

pub struct Spinner {
    pub active: Mutable<bool>,
    fullscreen: Mutable<bool>,
}

impl Spinner {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            active: Mutable::new(false),
            fullscreen: Mutable::new(false),
        })
    }

    pub fn set_active(&self, active: bool) {
        self.active.set_neq(active);
    }

    pub fn set_fullscreen(&self, fullscreen: bool) {
        self.fullscreen.set_neq(fullscreen);
    }

    pub fn signal(&self) -> impl Signal<Item = bool> {
        self.active.signal_ref(|x| *x)
    }

    pub fn render(spinner: &Self) -> Dom {
        html!("div", {
            .class_signal([
                "w-full",
                "h-full",
                "fixed",
                "flex",
                "top-0",
                "left-0",
                "bg-white",
                "dark:bg-gray-900",
                "bg-opacity-50",
                "z-10",
                "justify-center",
            ], spinner.fullscreen.signal_cloned().map(|x| x))
            .class_signal([
                "w-full",
                "block",
                "flex",
                "mx-auto",
                "z-10",
                "justify-center",
            ], spinner.fullscreen.signal_cloned().map(|x| !x))
            .visible_signal(spinner.active.signal_cloned().map(|x| x))
            .children(&mut [
                html!("div", {
                    .class(["loader-dots", "block", "relative", "w-20", "h-5", "m-auto"])
                    .children(&mut [
                        html!("div", {
                            .class(["absolute", "top-0", "mt-1", "w-3", "h-3", "rounded-full", "bg-accent"])
                        }),
                        html!("div", {
                            .class(["absolute", "top-0", "mt-1", "w-3", "h-3", "rounded-full", "bg-accent"])
                        }),
                        html!("div", {
                            .class(["absolute", "top-0", "mt-1", "w-3", "h-3", "rounded-full", "bg-accent"])
                        }),
                        html!("div", {
                            .class(["absolute", "top-0", "mt-1", "w-3", "h-3", "rounded-full", "bg-accent"])
                        }),
                    ])
                })
            ])
        })
    }
}
