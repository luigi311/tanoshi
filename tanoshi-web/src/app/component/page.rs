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
                class={if self.props.hidden {
                    "w-auto min-h-24 object-contain hidden"
                } else {
                    "w-auto min-h-24 object-contain block"
                }}
                src={&self.props.src}
                onmouseup={self.link.callback(|_| Msg::MouseUp)}
                style={"background: transparent url('/assets/loading.gif') no-repeat scroll center center"}
            />
        }
    }
}
