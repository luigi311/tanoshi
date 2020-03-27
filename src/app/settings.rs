use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use super::component::model::{
    BackgroundColor, ChapterModel, GetChaptersResponse, GetMangaResponse, MangaModel,
    PageRendering, ReadingDirection,
};

use crate::app::component::model::SettingParams;
use anyhow;
use yew::services::storage::Area;
use yew::services::StorageService;

#[derive(Clone, Properties)]
pub struct Props {}

pub struct Settings {
    link: ComponentLink<Self>,
    storage: StorageService,
    settings: SettingParams,
}

pub enum Msg {
    SetReadingDirection(ReadingDirection),
    SetBackgroundColor(BackgroundColor),
    SetPageRendering(PageRendering),
    noop,
}

impl Component for Settings {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let settings = {
            if let Ok(settings) = storage.restore("settings") {
                serde_json::from_str(settings.as_str()).expect("failed to serialize")
            } else {
                SettingParams::default()
            }
        };
        Settings {
            link,
            storage,
            settings,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetReadingDirection(value) => {
                self.settings.reading_direction = value;
                self.storage.store("settings", &self.settings)
            }
            Msg::SetBackgroundColor(value) => {
                self.settings.background_color = value;
                self.storage.store("settings", &self.settings)
            }
            Msg::SetPageRendering(value) => {
                self.settings.page_rendering = value;
                self.storage.store("settings", &self.settings)
            }
            Msg::noop => {
                info!("noop");
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container pb-20" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
            <div class="flex flex-col rounded-lg border border-grey-light m-2" id="updates">
                <div class="flex justify-between border-b border-gray-light p-2 content-center">
                    <span class="font-semibold">{"Reading Direction"}</span>
                    <div class="inline-flex">
                        <button class={
                            format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-l",
                            if self.settings.reading_direction == ReadingDirection::RightToLeft { "bg-gray-400" } else {"bg-gray-300"})}
                            onclick=self.link.callback(|_| Msg::SetReadingDirection(ReadingDirection::RightToLeft))>
                            {"Right to Left"}
                        </button>
                        <button class={
                            format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r",
                            if self.settings.reading_direction == ReadingDirection::LeftToRight { "bg-gray-400" } else {"bg-gray-300"})}
                            onclick=self.link.callback(|_| Msg::SetReadingDirection(ReadingDirection::LeftToRight))>
                            {"Left to Right"}
                        </button>
                    </div>
                </div>
                <div class="flex justify-between border-b border-gray-light p-2 content-center">
                    <span class="font-semibold">{"Background Color"}</span>
                    <div class="inline-flex">
                        <button class={
                            format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-l",
                            if self.settings.background_color == BackgroundColor::White { "bg-gray-400" } else {"bg-gray-300"})}
                            onclick=self.link.callback(|_| Msg::SetBackgroundColor(BackgroundColor::White))>
                            {"White"}
                        </button>
                        <button class={
                            format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r",
                            if self.settings.background_color == BackgroundColor::Black { "bg-gray-400" } else {"bg-gray-300"})}
                            onclick=self.link.callback(|_| Msg::SetBackgroundColor(BackgroundColor::Black))>
                            {"Black"}
                        </button>
                    </div>
                </div>
                <div class="flex justify-between border-b border-gray-light p-2 content-center ">
                    <span class="font-semibold">{"Page Rendering"}</span>
                    <div class="inline-flex">
                        <button class={
                            format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-l",
                            if self.settings.page_rendering == PageRendering::SinglePage { "bg-gray-400" } else {"bg-gray-300"})}
                            onclick=self.link.callback(|_| Msg::SetPageRendering(PageRendering::SinglePage))>
                            {"Single"}
                        </button>
                        <button class={
                            format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4",
                            if self.settings.page_rendering == PageRendering::DoublePage { "bg-gray-400" } else {"bg-gray-300"})}
                            onclick=self.link.callback(|_| Msg::SetPageRendering(PageRendering::DoublePage))>
                            {"Double"}
                        </button>
                        <button class={
                            format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r",
                            if self.settings.page_rendering == PageRendering::LongStrip { "bg-gray-400" } else {"bg-gray-300"})}
                            onclick=self.link.callback(|_| Msg::SetPageRendering(PageRendering::LongStrip))>
                            {"Long Strip"}
                        </button>
                    </div>
                </div>
            </div>
            </div>
        }
    }
}

impl Settings {}
