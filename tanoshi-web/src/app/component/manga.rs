use std::time::Duration;
use web_sys::Node;
use yew::prelude::*;
use yew::services::{Task, TimeoutService};
use yew::virtual_dom::VNode;
use yew::{
    html, Component, ComponentLink, Html, Properties, ShouldRender,
};

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub id: i32,
    pub title: String,
    pub thumbnail: String,
    pub is_favorite: bool,
    #[prop_or_default]
    pub on_tap: Callback<()>,
    #[prop_or_default]
    pub on_long_tap: Callback<()>,
}

pub struct Manga {
    link: ComponentLink<Self>,
    job: Option<Box<dyn Task>>,
    props: Props,
    is_dragging: bool,
}

pub enum Msg {
    Click(MouseEvent),
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseDownTimeout,
    TouchStart(TouchEvent),
    TouchEnd(TouchEvent),
    TouchMove(TouchEvent),
}

impl Component for Manga {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Manga {
            link,
            job: None,
            props,
            is_dragging: false,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            true
        } else {
            false
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Click(e) => {
                e.prevent_default();
                return false;
            }
            Msg::MouseDown(e) => {
                e.prevent_default();
                self.start_timer();
            }
            Msg::MouseUp(e) => {
                e.prevent_default();
                if !self.is_dragging && self.job.is_some() {
                    self.props.on_tap.emit(());
                    self.job = None;
                }
            }
            Msg::TouchStart(e) => {
                e.prevent_default();
                self.start_timer();
                self.is_dragging = false;
            }
            Msg::TouchEnd(e) => {
                e.prevent_default();
                if !self.is_dragging && self.job.is_some() {
                    self.props.on_tap.emit(());
                    self.job = None;
                }
            }
            Msg::TouchMove(e) => {
                e.prevent_default();
                self.is_dragging = true;
            }
            Msg::MouseDownTimeout => {
                self.job = None;
                self.props.on_long_tap.emit(());
            }
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
        let _ = div.set_inner_html(&self.props.title);

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
                <img class="absolute w-full h-full object-cover rounded-md" src={self.props.thumbnail.to_owned()} />
                {vnode}
            </div>
        }
    }
}

impl Manga {
    fn classes(&self) -> Vec<&str> {
        let mut classes = vec!["cursor-pointer", "relative", "rounded-md", "pb-7/5"];
        if self.props.is_favorite {
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
