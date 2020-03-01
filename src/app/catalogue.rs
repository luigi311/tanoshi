use serde::{Deserialize};
use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

use super::component::{Manga};
use stdweb::web::{IEventTarget, window, document, IHtmlElement};
use stdweb::web::event::ScrollEvent;
use enclose::enclose;
use std::borrow::BorrowMut;
use crate::app::{GetMangasResponse, MangaModel};


#[derive(Clone, Properties)]
pub struct Props {
    pub source: String,
}

pub struct Catalogue {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    source: String,
    page: i32,
    mangas: Vec<MangaModel>,
    is_fetching: bool,
}

pub enum Msg {
    MangaReady(GetMangasResponse),
    ScrolledDown,
    Noop,
}

impl Component for Catalogue {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let scroll_callback = link.callback(move |_ : ScrollEvent| {
            let current_scroll = window().page_y_offset() as i32 + window().inner_height();
            let height = document().body().unwrap().offset_height();
            if current_scroll >= height {
                info!("scroll end");
               return Msg::ScrolledDown;
            }
            Msg::Noop
        });
        window().add_event_listener(enclose!((window) move |e: ScrollEvent| {
            scroll_callback.emit(e)
        }));
        Catalogue {
            fetch_task: None,
            link,
            source: props.source,
            page: 1,
            mangas: vec![],
            is_fetching: false,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(mut data) => {
                let mut mangas = data.mangas;
                self.mangas.append(&mut mangas);
                self.is_fetching = false;
            }
            Msg::ScrolledDown => {
                if !self.is_fetching {
                    self.page += 1;
                    self.fetch_mangas();
                }
            }
            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn mounted(&mut self) -> ShouldRender {
        self.fetch_mangas();
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container-fluid">
                <div class="row row-cols-sm-2 row-cols-md-3 row-cols-lg-5 row-cols-xl-6">
                { for self.mangas.iter().map(|manga|  html!{
                <Manga
                    title=manga.title.to_owned()
                    thumbnail=manga.thumbnail_url.to_owned()
                    path=manga.path.to_owned()
                    source=self.source.to_owned() />
                }) }
                </div>
            </div>
        }
    }
}

impl Catalogue {
    fn fetch_mangas(&mut self) {
        let req = Request::get(format!("/api/source/{}?sort_by=popularity&sort_order=descending&page={}", self.source, self.page))
            .body(Nothing)
            .expect("failed to build request");

        let task = FetchService::new().fetch(
            req,
            self.link.callback(|response: Response<Json<Result<GetMangasResponse, anyhow::Error>>>| {
                if let (meta, Json(Ok(data))) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::MangaReady(data);
                    }
                }
                Msg::Noop
            }));
        self.fetch_task = Some(task);
        self.is_fetching = true;
    }
}
