use serde::{Deserialize, Serialize};
use yew::format::{Json, Nothing};
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub show: bool,
    #[prop_or_default]
    pub on_sort_by_change: Callback<InputData>,
    #[prop_or_default]
    pub on_sort_direction_change: Callback<InputData>,
}

pub struct Filter {
    link: ComponentLink<Self>,
    props: Props,
    node_ref: NodeRef,
}

pub enum Msg {}

impl Component for Filter {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Filter {
            link,
            props,
            node_ref: NodeRef::default(),
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
        false
    }

    fn view(&self) -> Html {
        html! {
            <div ref={self.node_ref.clone()} class={self.classes()}>
                <span>{"test modal fitler"}</span>
            </div>
        }
    }
}

impl Filter {
    fn classes(&self) -> Vec<&str> {
        let mut classes = vec![
            "animated",
            "block",
            "fixed",
            "pb-1/2",
            "z-25",
            "inset-x-0",
            "bottom-0",
            "mx-auto",
            "h-1/2",
            "w-full",
            "md:w-1/2",
            "lg:w-1/2",
            "px-4",
            "rounded-t-md",
            "bg-white",
            "shadow-top",
            "safe-bottom",
        ];
        if self.props.show {
            classes.push("slideInUp");
        } else {
            classes.push("slideOutDown");
        }
        classes
    }
}
