use serde::{Deserialize, Serialize};
use yew::worker::*;

use tanoshi_lib::manga::HistoryRequest;
use yew::format::{Json, Text};
use yew::services::fetch::{FetchService, FetchTask};

#[derive(Deserialize, Serialize)]
pub enum Request {
    PostHistory(String, HistoryRequest),
}

#[derive(Deserialize, Serialize)]
pub enum Response {
    HistoryPosted,
}

pub struct Worker {
    link: AgentLink<Worker>,
    fetch_task: Option<FetchTask>,
}

pub enum Msg {
    HistoryPosted(HandlerId),
    Noop,
}

impl Agent for Worker {
    type Reach = Job;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        Worker {
            link,
            fetch_task: None,
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::HistoryPosted(id) => {
                self.link.respond(id, Response::HistoryPosted);
            }
            Msg::Noop => {}
        }
    }

    fn handle_input(&mut self, msg: Self::Input, id: HandlerId) {
        match msg {
            Request::PostHistory(token, request) => {
                let req = http::request::Request::post("/api/history")
                    .header("Authorization", token.to_string())
                    .header("Content-Type", "application/json")
                    .body(Json(&request))
                    .expect("failed to build request");

                if let Ok(task) = FetchService::new().fetch(
                    req,
                    self.link
                        .callback(move |response: http::response::Response<Text>| {
                            if let (meta, Ok(data)) = response.into_parts() {
                                if meta.status.is_success() {
                                    return Msg::HistoryPosted(id);
                                }
                            }
                            Msg::Noop
                        }),
                ) {
                    self.fetch_task = Some(FetchTask::from(task));
                }
            }
        }
    }
}
