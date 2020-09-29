use serde::{Deserialize, Serialize};
use yew::services::fetch::FetchTask;
use yew::worker::{Agent, AgentLink, HandlerId, Job};
use crate::app::api;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    FetchMangaDetail(i32),
    FetchMangaChapters(i32),
    Favorite(i32, usize, bool),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    DataFetched,
    Favorited(usize, bool),
}

pub enum Msg {
    Initialized,
    MangaDetailFetched(HandlerId),
    MangaChaptersFetched(HandlerId),
    Favorited(HandlerId, usize, bool),
}

pub struct Worker {
    link: AgentLink<Worker>,
    fetch_task: HashMap<String, FetchTask>,
}

impl Agent for Worker {
    type Reach = Job<Self>;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        link.send_message(Msg::Initialized);
        Self {
            link,
            fetch_task: HashMap::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::Initialized => {
                log::info!("Initialized!");
            }
            Msg::MangaDetailFetched(who) => {
                log::info!("Data was fetched");
                self.fetch_task.remove(format!("detail#{:?}", who).as_str());
                self.link.respond(who, Response::DataFetched);
            }
            Msg::MangaChaptersFetched(who) => {
                log::info!("Data was fetched");
                self.fetch_task.remove(format!("chapters#{:?}", who).as_str());
                self.link.respond(who, Response::DataFetched);
            }
            Msg::Favorited(who, index, favorite) => {
                self.fetch_task.remove(format!("favorite#{:?}", who).as_str());
                self.link.respond(who, Response::Favorited(index, favorite));
            }
        }
    }

    fn handle_input(&mut self, msg: Self::Input, who: HandlerId) {
        match msg {
            Request::FetchMangaDetail(manga_id) => {
                self.fetch_task.insert(format!("detail#{:?}", who), api::fetch_manga(
                    manga_id,
                    self.link.callback(move |_| Msg::MangaDetailFetched(who))
                ).unwrap());
            }
            Request::FetchMangaChapters(manga_id) => {
                self.fetch_task.insert(format!("chapters#{:?}", who), api::fetch_chapters(
                    manga_id,
                    false,
                    self.link.callback(move |_| Msg::MangaChaptersFetched(who))
                ).unwrap());
            }
            Request::Favorite(manga_id, index, favorite) => {
                let task = if favorite {
                    api::favorite(manga_id, self.link.callback(move |_| Msg::Favorited(who, index, favorite))).unwrap()
                } else {
                    api::unfavorite(manga_id, self.link.callback(move |_| Msg::Favorited(who, index, favorite))).unwrap()
                };
                self.fetch_task.insert(format!("favorite#{:?}", who), task);
            }
        }
    }
}