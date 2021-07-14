use std::rc::Rc;

use dominator::{Dom, html};
use futures_signals::signal::Mutable;

pub struct  Snackbar {
    pub show: Mutable<bool>,
}

impl Snackbar {
    pub fn new() -> Rc<Self> {
        Rc::new(Self{
            show: Mutable::new(false)
        })
    }

    pub fn show(message: String) {
        Self::show_with_timeout(message, 3000);
    }

    pub fn show_with_timeout(message: String, duration: i64) {
        
    }

    pub fn render(snackbar: Rc<Self>) -> Dom {
        html!("div", {
            
        })
    }
}