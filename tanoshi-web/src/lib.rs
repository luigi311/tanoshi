#![recursion_limit = "2048"]
extern crate chrono;
extern crate log;
extern crate wee_alloc;

mod utils;
use wasm_bindgen::prelude::*;
use wasm_logger;

mod app;

// allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    utils::set_panic_hook();
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<app::App>();
    Ok(())
}
