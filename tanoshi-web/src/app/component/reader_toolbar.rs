use crate::app::browse::BrowseRoute;
use crate::app::AppRoute;
use web_sys::{HtmlElement, MouseEvent};
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub visible: bool,
    pub manga_id: i32,
    pub title: String,
    pub chapter: String,
    pub on_refresh: Callback<MouseEvent>,
}

pub struct ReaderToolbar {
    props: Props,
    root_ref: NodeRef,
    title_ref: NodeRef,
}

pub enum Msg {}

impl Component for ReaderToolbar {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        ReaderToolbar {
            props,
            root_ref: NodeRef::default(),
            title_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, _msg: Self::Message) -> bool {
        false
    }

    fn change(&mut self, props: Self::Properties) -> bool {
        if self.props != props {
            self.props = props;

            if let Some(title) = self.title_ref.cast::<HtmlElement>() {
                let _ = title.set_inner_html(self.props.title.as_str());
            }
            if !self.props.visible {
                if let Some(bar) = self.root_ref.cast::<HtmlElement>() {
                    bar.class_list()
                        .remove_1("slideInDown")
                        .expect("failed remove class");
                    bar.class_list()
                        .add_1("slideOutUp")
                        .expect("failed add class");
                }
            } else {
                if let Some(bar) = self.root_ref.cast::<HtmlElement>() {
                    bar.class_list()
                        .remove_1("slideOutUp")
                        .expect("failed remove class");
                    bar.class_list()
                        .add_1("slideInDown")
                        .expect("failed add class");
                }
            }
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        html! {
            <div ref=self.root_ref.clone()
                class="flex justify-between items-center animated slideInDown faster block fixed inset-x-0 top-0 z-50 bg-gray-900 z-50 content-end opacity-75"
                style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                <RouterAnchor<AppRoute> classes="z-50 mx-2 mb-2 text-white" route=AppRoute::Browse(BrowseRoute::Detail(self.props.manga_id))>
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </RouterAnchor<AppRoute>>
                <div class="flex flex-col mx-2 mb-2">
                    <span ref=self.title_ref.clone() class="text-white text-center"></span>
                    <span class="text-white text-center text-sm">{&self.props.chapter}</span>
                </div>
                <button
                    onclick={&self.props.on_refresh}
                    class="z-50 mx-2 mb-2 text-white ">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                </button>
            </div>
        }
    }
}
