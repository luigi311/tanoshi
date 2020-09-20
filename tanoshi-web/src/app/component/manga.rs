use std::time::Duration;
use web_sys::Node;
use yew::format::{Json};
use yew::prelude::*;
use yew::services::fetch::{FetchTask};
use yew::services::{Task, TimeoutService};
use yew::virtual_dom::VNode;
use yew::{
    html, Bridge, Bridged, Children, Component, ComponentLink, Html, Properties, ShouldRender,
};
use yew_router::agent::{RouteAgent, RouteRequest};
use yew_router::prelude::*;

use crate::app::api;
use tanoshi_lib::rest::AddFavoritesResponse;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub id: i32,
    pub title: String,
    pub thumbnail: String,
    pub is_favorite: bool,
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub on_to_detail: Callback<()>,
}

pub struct Manga {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    job: Option<Box<dyn Task>>,
    id: i32,
    router: Box<dyn Bridge<RouteAgent>>,
    title: String,
    thumbnail: String,
    is_favorite: bool,
    is_dragging: bool,
    on_to_detail: Callback<()>,
}

pub enum Msg {
    Click(MouseEvent),
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseDownTimeout,
    TouchStart(TouchEvent),
    TouchEnd(TouchEvent),
    TouchMove(TouchEvent),
    Favorited(AddFavoritesResponse),
    Unfavorited(AddFavoritesResponse),
    Noop,
}

impl Component for Manga {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let callback = link.callback(|_| Msg::Noop);
        let router = RouteAgent::bridge(callback);
        Manga {
            fetch_task: None,
            link,
            job: None,
            router,
            id: props.id,
            title: props.title,
            thumbnail: props.thumbnail,
            is_favorite: props.is_favorite,
            is_dragging: false,
            on_to_detail: props.on_to_detail,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn rendered(&mut self, _first_render: bool) {}

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Click(e) => {
                e.prevent_default();
            }
            Msg::MouseDown(_e) => {
                self.start_timer();
            }
            Msg::MouseUp(e) => {
                e.prevent_default();
                self.on_to_detail.emit(());
                self.to_detail();
            }
            Msg::TouchStart(_e) => {
                self.start_timer();
                self.is_dragging = false;
            }
            Msg::TouchEnd(e) => {
                e.prevent_default();
                if !self.is_dragging {
                    self.to_detail();
                }
            }
            Msg::TouchMove(_e) => {
                self.is_dragging = true;
            }
            Msg::MouseDownTimeout => {
                if !self.is_dragging {
                    if self.is_favorite {
                        self.unfavorite();
                    } else {
                        self.favorite();
                    }
                }
                self.job = None;
            }
            Msg::Favorited(data) => {
                if data.status == "success" {
                    self.is_favorite = true;
                }
            }
            Msg::Unfavorited(data) => {
                if data.status == "success" {
                    self.is_favorite = false;
                }
            }
            Msg::Noop => return false,
        }
        true
    }

    fn view(&self) -> Html {
        let div = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("span")
            .unwrap();
        let _ = div.class_list().add_6(
            "absolute",
            "bottom-0",
            "sm:text-sm",
            "text-xs",
            "bg-black",
            "w-full",
        );
        let _ = div.class_list().add_5(
            "opacity-75",
            "text-white",
            "p-1",
            "truncate",
            "rounded-b-md",
        );
        let _ = div.set_inner_html(&self.title);

        let node = Node::from(div);
        let vnode = VNode::VRef(node);
        html! {
            <div
                class=self.classes()
                onmousedown=self.link.callback(|e| Msg::MouseDown(e))
                onmouseup=self.link.callback(|e| Msg::MouseUp(e))
                ontouchstart=self.link.callback(|e| Msg::TouchStart(e))
                ontouchend=self.link.callback(|e| Msg::TouchEnd(e))
                ontouchmove=self.link.callback(|e| Msg::TouchMove(e))
                onclick=self.link.callback(|e| Msg::Click(e))
            >
                <img class="absolute w-full h-full object-cover rounded-md" src={self.thumbnail.to_owned()} />
                {vnode}
            </div>
        }
    }
}

impl Manga {
    fn classes(&self) -> Vec<&str> {
        let mut classes = vec!["cursor-pointer", "relative", "rounded-md", "pb-7/5"];
        if self.is_favorite {
            classes.push("favorite");
        }
        return classes;
    }

    fn favorite(&mut self) {
        if let Ok(task) = api::favorite(
            self.id,
            self.link.callback(
                |response: api::FetchJsonResponse<AddFavoritesResponse>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::Favorited(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }

    fn unfavorite(&mut self) {
        if let Ok(task) = api::unfavorite(
            self.id,
            self.link.callback(
                |response: api::FetchJsonResponse<AddFavoritesResponse>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::Unfavorited(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
    fn start_timer(&mut self) {
        let handle = TimeoutService::spawn(
            Duration::from_secs(1),
            self.link.callback(|_| Msg::MouseDownTimeout),
        );
        self.job = Some(Box::new(handle));
    }

    fn to_detail(&mut self) {
        if !self.job.is_none() {
            self.router
                .send(RouteRequest::ChangeRoute(Route::from(format!(
                    "/manga/{}",
                    self.id
                ))));
            self.job = None;
        }
    }
}
