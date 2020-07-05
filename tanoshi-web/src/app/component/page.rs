use super::model::{PageRendering, ReadingDirection};
use yew::html::{Children, NodeRef};
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub id: usize,
    #[prop_or_default]
    pub src: String,
    #[prop_or_default]
    pub onmouseup: Callback<()>,
    #[prop_or_default]
    pub hidden: bool,
    #[prop_or_default]
    pub children: Children,
    pub page_ref: NodeRef,
    pub page_rendering: PageRendering,
    pub reading_direction: ReadingDirection,
}
pub struct Page {
    props: Props,
    link: ComponentLink<Self>,
}

pub enum Msg {
    MouseUp,
}

impl Component for Page {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Page { props, link }
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
            Msg::MouseUp => self.props.onmouseup.emit(()),
        }
        false
    }

    fn view(&self) -> Html {
        html! {
            <img id={self.props.id}
                ref=self.props.page_ref.clone(),
                class={self.classess()}
                src={&self.props.src}
                onmouseup={self.link.callback(|_| Msg::MouseUp)}
                style={"background: transparent url('/assets/loading.gif') no-repeat scroll center center"}
            />
        }
    }
}

impl Page {
    fn classess(&self) -> Vec<&str> {
        let mut classes = vec!["object-contain"];
        if self.props.hidden {
            classes.push("hidden");
        } else {
            classes.push("block");
        }
        match self.props.page_rendering {
            PageRendering::SinglePage => {
                classes.push("w-full");
                classes.push("h-auto");
            }
            PageRendering::DoublePage => {
                classes.push("w-1/2");
                classes.push("h-auto");
                if self.props.id % 2 == 0 {
                    match self.props.reading_direction {
                        ReadingDirection::RightToLeft => classes.push("object-left"),
                        ReadingDirection::LeftToRight => classes.push("object-right"),
                    }
                } else {
                    match self.props.reading_direction {
                        ReadingDirection::RightToLeft => classes.push("object-right"),
                        ReadingDirection::LeftToRight => classes.push("object-left"),
                    }
                }
            }
            PageRendering::LongStrip => {
                classes.push("w-auto");
                classes.push("min-h-24");
            }
        }

        classes
    }
}
