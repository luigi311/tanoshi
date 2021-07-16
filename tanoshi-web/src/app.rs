use std::rc::Rc;

use dominator::routing;
use dominator::{clone, html, Dom};
use futures_signals::signal::SignalExt;
use wasm_bindgen::UnwrapThrowExt;

use crate::catalogue::Catalogue;
use crate::common::snackbar;
use crate::library::Library;
use crate::login::Login;
use crate::manga::Manga;
use crate::reader::Reader;
use crate::utils::local_storage;
use crate::{
    common::{Bottombar, Route, Spinner},
    histories::Histories,
    settings::Settings,
    updates::Updates,
    utils::AsyncLoader,
};

pub struct App {
    pub spinner: Rc<Spinner>,
    pub loader: AsyncLoader,
}

impl App {
    pub fn new() -> Rc<Self> {
        Rc::new(App {
            spinner: Spinner::new(),
            loader: AsyncLoader::new(),
        })
    }

    pub fn render(app: Rc<Self>) -> Dom {
        match local_storage().get("token").unwrap_throw() {
            Some(_) => {},
            None => routing::go_to_url(&Route::Login.url()),
        };

        html!("div", {
            .child_signal(Route::signal().map(clone!(app => move |x| {
                match x {
                    Route::Login => Some(
                        Login::render(Login::new())
                    ),
                    Route::Library => Some(
                        Library::render(Library::new()),
                    ),
                    Route::Catalogue{id, latest} => Some(
                        Catalogue::render(Catalogue::new(), id, latest),
                    ),
                    Route::Manga(manga_id) => Some(
                        Manga::render(Manga::new(manga_id, 0, "".to_string())),
                    ),
                    Route::MangaBySourcePath(source_id, path) => Some(
                        Manga::render(Manga::new(0, source_id, path)),
                    ),
                    Route::Chapter(chapter_id, page) => Some(
                        Reader::render(Reader::new(chapter_id, page)),
                    ),
                    Route::Updates => Some(
                        Updates::render(Updates::new(), app.clone()),
                    ),
                    Route::Histories => Some(
                        Histories::render(Histories::new(), app.clone()),
                    ),
                    Route::Settings(category) => Some(
                        Settings::render(Settings::new(), category),
                    ),
                    Route::NotFound => Some(
                        html!("div", {
                            .text("not found")
                        }),
                    )
                }
            })))
            .child_signal(Route::signal().map(|x| {
                match x {
                    Route::Login | Route::Chapter(_, _) => None,
                    _ => Some(Bottombar::render()),
                }
            }))
            .children(&mut [
                snackbar::render(),
            ])
        })
    }
}
