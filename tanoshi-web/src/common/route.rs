use wasm_bindgen::prelude::*;
use web_sys::Url;
use futures_signals::signal::{Signal, SignalExt};
use dominator::routing;

#[derive(Debug, Clone)]
pub enum SettingCategory {
    None,
    Reader,
    Source(i64),
    Users,
    CreateUser,
    User,
}

#[derive(Debug)]
pub enum Route {
    Login,
    Library,
    Catalogue {
        id: i64,
        latest: bool,
    },
    Manga(i64),
    Chapter(i64),
    Updates,
    Histories,
    Settings(SettingCategory),
    NotFound,
}

impl Route {
    pub fn signal() -> impl Signal<Item = Self> {
        routing::url()
        .signal_ref(|url| Url::new(&url).unwrap_throw())
        .map(|url| {
            let pathname = url.pathname();
            let mut paths = pathname.split("/").collect::<Vec<_>>();
            paths.retain(|path| *path != "");

            match paths.as_slice() {
                ["login"] => Route::Login,
                [] => Route::Library,
                ["updates"] => Route::Updates,
                ["histories"] => Route::Histories,
                ["catalogue"] => Route::Catalogue{id: 0, latest: false},
                ["catalogue", id] => {
                    if let Ok(id) = id.parse() {
                        Route::Catalogue{id, latest: false}
                    } else {
                        Route::NotFound
                    }
                },
                ["catalogue", id, "latest"] => {
                    if let Ok(id) = id.parse() {
                        Route::Catalogue{id, latest: true}
                    } else {
                        Route::NotFound
                    }
                },
                ["manga", id] => {
                    if let Ok(id) = id.parse() {
                        Route::Manga(id)
                    } else {
                        Route::NotFound
                    }
                },
                ["chapter", id] => {
                    if let Ok(id) = id.parse() {
                        Route::Chapter(id)
                    } else {
                        Route::NotFound
                    }
                },
                ["settings"] => Route::Settings(SettingCategory::None),
                ["settings", cat] => {
                    match *cat {
                        "reader" => Route::Settings(SettingCategory::Reader),
                        "sources" => Route::Settings(SettingCategory::Source(0)),
                        "users" => Route::Settings(SettingCategory::Users),
                        "user" => Route::Settings(SettingCategory::User),
                        _ => Route::NotFound
                    }
                },
                ["settings", "users", cat] => {
                    match *cat {
                        "create" => Route::Settings(SettingCategory::CreateUser),
                        _ => Route::NotFound
                    }
                },
                ["settings", "sources", id] => {
                    if let Ok(id) = id.parse() {
                        Route::Settings(SettingCategory::Source(id))
                    } else {
                        Route::NotFound
                    }
                },
                _ => Route::NotFound
            }
        })
    }

    pub fn url(&self) -> String {
        match self {
            Route::Login => "/login".to_string(),
            Route::Library => "/".to_string(),
            Route::Catalogue{id, latest} => {
                if *id > 0 && *latest {
                    ["/catalogue".to_string(), id.to_string(), "latest".to_string()].join("/")
                } else if *id > 0 && !*latest {
                    ["/catalogue".to_string(), id.to_string()].join("/")
                } else {
                    "/catalogue".to_string()
                }
            },
            Route::Manga(manga_id) => ["/manga".to_string(), manga_id.to_string()].join("/"),
            Route::Chapter(chapter_id) => ["/chapter".to_string(), chapter_id.to_string()].join("/"),
            Route::Updates => "/updates".to_string(),
            Route::Histories => "/histories".to_string(),
            Route::Settings(SettingCategory::None) => "/settings".to_string(),
            Route::Settings(SettingCategory::Reader) => "/settings/reader".to_string(),
            Route::Settings(SettingCategory::Source(source_id)) => if *source_id > 0 {format!("/settings/sources/{}", source_id)} else {"/settings/sources".to_string()},
            Route::Settings(SettingCategory::Users) => "/settings/users".to_string(),
            Route::Settings(SettingCategory::CreateUser) => "/settings/users/create".to_string(),
            Route::Settings(SettingCategory::User) => "/settings/user".to_string(),
            Route::NotFound => "/notfound".to_string()
        }
    }
}