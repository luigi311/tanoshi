use dominator::{html, link, Dom};
use futures_signals::signal::Mutable;
use serde::{Deserialize, Serialize};

use crate::common::route::Route;
use crate::utils::proxied_image_url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cover {
    pub id: i64,
    pub title: String,
    pub cover_url: String,
    pub is_favorite: Mutable<bool>
}

impl Cover {
    pub fn new(id: i64, title: String, cover_url: String) -> Self {
        let cover_url = proxied_image_url(&cover_url);
        Self {
            id,
            title,
            cover_url,
            is_favorite: Mutable::new(false),
        }
    }

    pub fn set_favorite(&self, favorite: bool) {
        self.is_favorite.set(favorite);
    }

    pub fn render(&self) -> Dom {
        link!(Route::Manga(self.id).url(), {
            .class("cursor-pointer")
            .class("relative")
            .class("rounded-md")
            .class("pb-7/5")
            .class("shadow")
            .children(&mut [
                html!("img", {
                    .class("absolute")
                    .class("w-full")
                    .class("h-full")
                    .class("object-cover")
                    .class("rounded-md")
                    .attribute("src", &self.cover_url)
                    .attribute("loading", "lazy")
                }),
                html!("span", {
                    .class("absolute")
                    .class("bottom-0")
                    .class("sm:text-sm")
                    .class("text-xs")
                    .class("bg-gradient-to-t")
                    .class("from-gray-900")
                    .class("to-transparent")
                    .class("w-full")
                    .class("opacity-75")
                    .class("text-gray-50")
                    .class("px-1")
                    .class("pb-1")
                    .class("pt-4")
                    .class("truncate")
                    .class("rounded-b-md")
                    .text(&self.title)
                })
            ])
        })
    }
}
