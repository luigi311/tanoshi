use yew::prelude::*;
use yew::html::Children;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    #[prop_or_default]
    pub children: Children,
}

pub struct TopBar {
    props: Props,
}

pub enum Msg {

}

impl Component for TopBar {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        TopBar {
            props
        }
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        true
    }

    fn change(&mut self, props: Self::Properties) -> bool {
        if self.props != props {
            self.props = props;
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        html! {
            <div class="w-full px-2 pb-2 flex justify-between block fixed inset-x-0 top-0 z-50 bg-accent border-b border-accent-darker text-white" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
            {
                if self.props.children.is_empty() {
                    html! {}
                } else {
                    html! {<>{ self.props.children.clone() }</>}
                }
            }
            </div>
        }
    }
}
