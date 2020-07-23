use super::model::{PageRendering, ReadingDirection};
use web_sys::{HtmlElement, MouseEvent};
use yew::html::NodeRef;
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub id: usize,
    #[prop_or_default]
    pub src: String,
    #[prop_or_default]
    pub onmouseup: Callback<MouseEvent>,
    #[prop_or_default]
    pub hidden: bool,
    pub page_ref: NodeRef,
    pub page_rendering: PageRendering,
    pub reading_direction: ReadingDirection,
}

pub struct Page {
    props: Props,
    #[allow(dead_code)]
    link: ComponentLink<Self>,
}

pub enum Msg {
    ImageLoad,
}

impl Component for Page {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Page { props, link }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ImageLoad => {
                let img = self.props.page_ref.cast::<HtmlElement>().unwrap();
                img.class_list().remove_2("h-screen", "sm:h-page");
                img.class_list().add_1("h-auto");
                true
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props.hidden != props.hidden {
            self.props.hidden = props.hidden;
            true
        } else if (self.props.src != props.src) && (self.props.src == "") {
            self.props.src = props.src;
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        html! {
            <img id={self.props.id}
                ref=self.props.page_ref.clone(),
                onload={self.link.callback(|_| Msg::ImageLoad)}
                class={self.classess()}
                src={&self.props.src}
                onmouseup={&self.props.onmouseup}
                style={"background: transparent url(\"data:image/gif;base64,R0lGODlhEAALAPQAAP///1t0m+fr8eHl7fL09194nlt0m3mOra+7zpmpwdPa5HGHqYuduLS/0ZyrwtXc5nSJql53nY6fuu/x9ebq8Pj5+4CTsens8ff4+tDX4sLL2t3i6vT2+Ft0m1t0m1t0myH5BAkLAAAAIf4aQ3JlYXRlZCB3aXRoIGFqYXhsb2FkLmluZm8AIf8LTkVUU0NBUEUyLjADAQAAACwAAAAAEAALAAAFLSAgjmRpnqSgCuLKAq5AEIM4zDVw03ve27ifDgfkEYe04kDIDC5zrtYKRa2WQgAh+QQJCwAAACwAAAAAEAALAAAFJGBhGAVgnqhpHIeRvsDawqns0qeN5+y967tYLyicBYE7EYkYAgAh+QQJCwAAACwAAAAAEAALAAAFNiAgjothLOOIJAkiGgxjpGKiKMkbz7SN6zIawJcDwIK9W/HISxGBzdHTuBNOmcJVCyoUlk7CEAAh+QQJCwAAACwAAAAAEAALAAAFNSAgjqQIRRFUAo3jNGIkSdHqPI8Tz3V55zuaDacDyIQ+YrBH+hWPzJFzOQQaeavWi7oqnVIhACH5BAkLAAAALAAAAAAQAAsAAAUyICCOZGme1rJY5kRRk7hI0mJSVUXJtF3iOl7tltsBZsNfUegjAY3I5sgFY55KqdX1GgIAIfkECQsAAAAsAAAAABAACwAABTcgII5kaZ4kcV2EqLJipmnZhWGXaOOitm2aXQ4g7P2Ct2ER4AMul00kj5g0Al8tADY2y6C+4FIIACH5BAkLAAAALAAAAAAQAAsAAAUvICCOZGme5ERRk6iy7qpyHCVStA3gNa/7txxwlwv2isSacYUc+l4tADQGQ1mvpBAAIfkECQsAAAAsAAAAABAACwAABS8gII5kaZ7kRFGTqLLuqnIcJVK0DeA1r/u3HHCXC/aKxJpxhRz6Xi0ANAZDWa+kEAA7\") no-repeat scroll center center"}
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
                classes.push("h-screen");
                classes.push("sm:h-page");
                classes.push("cursor-pointer");
            }
        }

        classes
    }
}
