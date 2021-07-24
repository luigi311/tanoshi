use dominator::{html, link, Dom};
use futures_signals::signal::Mutable;
use futures_signals::signal::Signal;
use serde::{Deserialize, Serialize};

use crate::common::route::Route;
use crate::utils::proxied_image_url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cover {
    pub id: i64,
    pub source_id: i64,
    pub path: String,
    pub title: String,
    pub cover_url: String,
    pub is_favorite: Mutable<bool>,
}

impl Cover {
    pub fn new(
        id: i64,
        source_id: i64,
        path: String,
        title: String,
        cover_url: String,
        is_favorite: bool,
    ) -> Self {
        let cover_url = proxied_image_url(&cover_url);
        Self {
            id,
            source_id,
            path,
            title,
            cover_url,
            is_favorite: Mutable::new(is_favorite),
        }
    }

    pub fn set_favorite(&self, favorite: bool) {
        self.is_favorite.set(favorite);
    }

    fn link(&self) -> String {
        if self.id != 0 {
            Route::Manga(self.id).url()
        } else if self.source_id != 0 && self.path != "" {
            Route::MangaBySourcePath(self.source_id, self.path.clone()).url()
        } else {
            Route::NotFound.url()
        }
    }

    pub fn render(&self) -> Dom {
        link!(self.link(), {
            .class("manga-cover")
            .class_signal(["filter", "brightness-50"], self.is_favorite.signal())
            .children(&mut [
                html!("img", {
                    .attribute("src", &self.cover_url)
                    .attribute("loading", "lazy")
                }),
                html!("span", {
                    // .class("absolute")
                    // .class("bottom-0")
                    // .class("sm:text-sm")
                    // .class("text-xs")
                    // .class("bg-gradient-to-t")
                    // .class("from-gray-900")
                    // .class("to-transparent")
                    // .class("w-full")
                    // .class("opacity-75")
                    // .class("text-gray-50")
                    // .class("px-1")
                    // .class("pb-1")
                    // .class("pt-4")
                    // .class("truncate")
                    // .class("rounded-b-md")
                    .text(&self.title)
                })
            ])
        })
    }
}
