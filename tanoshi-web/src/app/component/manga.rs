use std::time::Duration;
use web_sys::Node;
use yew::prelude::*;
use yew::services::{Task, TimeoutService};
use yew::virtual_dom::VNode;
use yew::{
    html, Children, Component, ComponentLink, Html, Properties, ShouldRender,
};

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub id: i32,
    pub title: String,
    pub thumbnail: String,
    pub is_favorite: bool,
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub on_tap: Callback<()>,
    #[prop_or_default]
    pub on_long_tap: Callback<()>,
}

pub struct Manga {
    link: ComponentLink<Self>,
    job: Option<Box<dyn Task>>,
    id: i32,
    title: String,
    thumbnail: String,
    is_favorite: bool,
    is_dragging: bool,
    on_tap: Callback<()>,
    on_long_tap: Callback<()>,
}

pub enum Msg {
    Click(MouseEvent),
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseDownTimeout,
    TouchStart(TouchEvent),
    TouchEnd(TouchEvent),
    TouchMove(TouchEvent),
    Noop,
}

impl Component for Manga {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Manga {
            link,
            job: None,
            id: props.id,
            title: props.title,
            thumbnail: props.thumbnail,
            is_favorite: props.is_favorite,
            is_dragging: false,
            on_tap: props.on_tap,
            on_long_tap: props.on_long_tap,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.is_favorite != props.is_favorite {
            self.is_favorite = props.is_favorite;
            true
        } else {
            false
        }
    }

    fn rendered(&mut self, _first_render: bool) {}

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Click(e) => {
                log::info!("Click");
                e.prevent_default();
                return false;
            }
            Msg::MouseDown(e) => {
                log::info!("MouseDown");
                e.prevent_default();
                self.start_timer();
            }
            Msg::MouseUp(e) => {
                log::info!("MouseUp");
                e.prevent_default();
                if !self.is_dragging && self.job.is_some() {
                    self.on_tap.emit(());
                    self.job = None;
                }
            }
            Msg::TouchStart(e) => {
                log::info!("TouchStart");
                e.prevent_default();
                self.start_timer();
                self.is_dragging = false;
            }
            Msg::TouchEnd(e) => {
                log::info!("TouchEnd");
                e.prevent_default();
                if !self.is_dragging && self.job.is_some() {
                    self.on_tap.emit(());
                    self.job = None;
                }
            }
            Msg::TouchMove(e) => {
                log::info!("TouchMove");
                e.prevent_default();
                self.is_dragging = true;
            }
            Msg::MouseDownTimeout => {
                log::info!("MouseDownTimeout");
                self.job = None;
                self.on_long_tap.emit(());
            }
            Msg::Noop => return false,
        }
        false
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

    fn start_timer(&mut self) {
        let handle = TimeoutService::spawn(
            Duration::from_secs(1),
            self.link.callback(|_| Msg::MouseDownTimeout),
        );
        self.job = Some(Box::new(handle));
    }
}
