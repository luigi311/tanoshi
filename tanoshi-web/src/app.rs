use std::rc::Rc;

use dominator::routing;
use dominator::{clone, html, Dom};
use futures_signals::signal::SignalExt;
use wasm_bindgen::UnwrapThrowExt;

use crate::catalogue::Catalogue;
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
    pub library_page: Rc<Library>,
    pub catalogue_page: Rc<Catalogue>,
    pub settings_page: Rc<Settings>,
}

impl App {
    pub fn new() -> Rc<Self> {
        Rc::new(App {
            spinner: Spinner::new(),
            loader: AsyncLoader::new(),
            library_page: Library::new(),
            catalogue_page: Catalogue::new(),
            settings_page: Settings::new(),
        })
    }

    pub fn render(app: Rc<Self>) -> Dom {
        match local_storage().get("token").unwrap_throw() {
            Some(_) => {},
            None => routing::go_to_url(&Route::Login.url()),
        };

        html!("div", {
            .children_signal_vec(Route::signal().map(clone!(app => move |x| {
                match x {
                    Route::Login => vec![
                        Login::render(Login::new())
                    ],
                    Route::Library => vec![
                        Library::render(app.library_page.clone()),
                        Bottombar::render()
                    ],
                    Route::Catalogue{id, latest} => vec![
                        Catalogue::render(app.catalogue_page.clone(), id, latest),
                        Bottombar::render()
                    ],
                    Route::Manga(manga_id) => vec![
                        Manga::render(Manga::new(manga_id, 0, "".to_string())),
                    ],
                    Route::MangaBySourcePath(source_id, path) => vec![
                        Manga::render(Manga::new(0, source_id, path)),
                    ],
                    Route::Chapter(chapter_id, page) => vec![
                        Reader::render(Reader::new(chapter_id, page)),
                    ],
                    Route::Updates => vec![
                        Updates::render(Updates::new(), app.clone()),
                        Bottombar::render()
                    ],
                    Route::Histories => vec![
                        Histories::render(Histories::new(), app.clone()),
                        Bottombar::render()
                    ],
                    Route::Settings(category) => vec![
                        Settings::render(app.settings_page.clone(), category),
                        Bottombar::render()
                    ],
                    Route::NotFound => vec![
                        html!("div", {
                            .text("not found")
                        }),
                        Bottombar::render()
                    ]
                }
            })).to_signal_vec())
        })
    }
}
