use crate::query::browse_source::{SortByParam, SortOrderParam};
use base64::{decode_config, encode_config, URL_SAFE_NO_PAD};
use dominator::routing;
use futures_signals::signal::{Signal, SignalExt};
use wasm_bindgen::prelude::*;
use web_sys::Url;

#[derive(Debug, Clone, Copy)]
pub enum SettingCategory {
    None,
    Appearance,
    Chapters,
    Reader,
    Source(i64),
    Users,
    CreateUser,
    User,
    ManageDownloads,
    DownloadQueue,
}

#[derive(Debug)]
pub enum Route {
    Login,
    Library,
    CatalogueList,
    Catalogue {
        id: i64,
        keyword: Option<String>,
        sort_by: SortByParam,
        sort_order: SortOrderParam,
    },
    Manga(i64),
    MangaBySourcePath(i64, String),
    Chapter(i64, i64),
    Updates,
    Histories,
    Settings(SettingCategory),
    NotFound,
}

impl Route {
    pub fn signal() -> impl Signal<Item = Self> {
        routing::url()
            .signal_ref(|url| Url::new(url).unwrap_throw())
            .map(|url| {
                let pathname = url.pathname();
                let mut paths = pathname.split('/').collect::<Vec<_>>();
                paths.retain(|path| !path.is_empty());

                match paths.as_slice() {
                    ["login"] => Route::Login,
                    [] => Route::Library,
                    ["updates"] => Route::Updates,
                    ["histories"] => Route::Histories,
                    ["catalogue"] => Route::CatalogueList,
                    ["catalogue", id] => {
                        if let Ok(id) = id.parse() {
                            let params = url.search_params();
                            let keyword = params.get("keyword");
                            let sort_by = params
                                .get("sort_by")
                                .and_then(|by| serde_plain::from_str(&by).ok())
                                .unwrap_or(SortByParam::VIEWS);
                            let sort_order = params
                                .get("sort_order")
                                .and_then(|order| serde_plain::from_str(&order).ok())
                                .unwrap_or(SortOrderParam::DESC);
                            Route::Catalogue {
                                id,
                                keyword,
                                sort_by,
                                sort_order,
                            }
                        } else {
                            Route::NotFound
                        }
                    }
                    ["manga", id] => {
                        if let Ok(id) = id.parse() {
                            Route::Manga(id)
                        } else {
                            Route::NotFound
                        }
                    }
                    ["manga", source_id, path] => {
                        if let Ok(id) = source_id.parse() {
                            if let Ok(decoded) = decode_config(path, URL_SAFE_NO_PAD) {
                                if let Ok(path) = String::from_utf8(decoded) {
                                    Route::MangaBySourcePath(id, path)
                                } else {
                                    Route::NotFound
                                }
                            } else {
                                Route::NotFound
                            }
                        } else {
                            Route::NotFound
                        }
                    }
                    ["chapter", id] => {
                        if let Ok(id) = id.parse() {
                            let page = url
                                .hash()
                                .strip_prefix('#')
                                .and_then(|h| h.parse().ok())
                                .unwrap_or(1);
                            info!("chapter {} page {}", id, page);
                            Route::Chapter(id, page - 1)
                        } else {
                            Route::NotFound
                        }
                    }
                    ["settings"] => Route::Settings(SettingCategory::None),
                    ["settings", cat] => match *cat {
                        "appearance" => Route::Settings(SettingCategory::Appearance),
                        "chapters" => Route::Settings(SettingCategory::Chapters),
                        "reader" => Route::Settings(SettingCategory::Reader),
                        "sources" => Route::Settings(SettingCategory::Source(0)),
                        "users" => Route::Settings(SettingCategory::Users),
                        "user" => Route::Settings(SettingCategory::User),
                        "downloads" => Route::Settings(SettingCategory::ManageDownloads),
                        "downloads-queue" => Route::Settings(SettingCategory::DownloadQueue),
                        _ => Route::NotFound,
                    },
                    ["settings", "users", "create"] => Route::Settings(SettingCategory::CreateUser),
                    ["settings", "sources", id] => {
                        if let Ok(id) = id.parse() {
                            Route::Settings(SettingCategory::Source(id))
                        } else {
                            Route::NotFound
                        }
                    }
                    _ => Route::NotFound,
                }
            })
    }

    pub fn url(&self) -> String {
        match self {
            Route::Login => "/login".to_string(),
            Route::Library => "/".to_string(),
            Route::CatalogueList => "/catalogue".to_string(),
            Route::Catalogue {
                id,
                keyword,
                sort_by,
                sort_order,
            } => {
                let sort_by = serde_plain::to_string(sort_by).unwrap();
                let sort_order = serde_plain::to_string(sort_order).unwrap();

                if let Some(keyword) = keyword {
                    format!(
                        "/catalogue/{}?keyword={}&sort_by={}&sort_order={}",
                        id, keyword, sort_by, sort_order
                    )
                } else {
                    format!(
                        "/catalogue/{}?sort_by={}&sort_order={}",
                        id, sort_by, sort_order
                    )
                }
            }
            Route::Manga(manga_id) => ["/manga".to_string(), manga_id.to_string()].join("/"),
            Route::MangaBySourcePath(source_id, path) => [
                "/manga".to_string(),
                source_id.to_string(),
                encode_config(path, URL_SAFE_NO_PAD),
            ]
            .join("/"),
            Route::Chapter(chapter_id, page) => {
                format!("/chapter/{}#{}", chapter_id, page + 1)
            }
            Route::Updates => "/updates".to_string(),
            Route::Histories => "/histories".to_string(),
            Route::Settings(SettingCategory::None) => "/settings".to_string(),
            Route::Settings(SettingCategory::Appearance) => "/settings/appearance".to_string(),
            Route::Settings(SettingCategory::Chapters) => "/settings/chapters".to_string(),
            Route::Settings(SettingCategory::Reader) => "/settings/reader".to_string(),
            Route::Settings(SettingCategory::Source(source_id)) => {
                if *source_id > 0 {
                    format!("/settings/sources/{}", source_id)
                } else {
                    "/settings/sources".to_string()
                }
            }
            Route::Settings(SettingCategory::Users) => "/settings/users".to_string(),
            Route::Settings(SettingCategory::CreateUser) => "/settings/users/create".to_string(),
            Route::Settings(SettingCategory::User) => "/settings/user".to_string(),
            Route::Settings(SettingCategory::ManageDownloads) => "/settings/downloads".to_string(),
            Route::Settings(SettingCategory::DownloadQueue) => {
                "/settings/downloads-queue".to_string()
            }
            Route::NotFound => "/notfound".to_string(),
        }
    }
}
