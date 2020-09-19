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
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" class="fill-current inline-block mb-1">
                        <path fill-rule="evenodd" d="M3 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z" clip-rule="evenodd"></path>
                    </svg>
               </RouterAnchor<AppRoute>>
               <div class="flex flex-col mx-2 mb-2">
                <span ref=self.title_ref.clone() class="text-white text-center"></span>
                <span class="text-white text-center text-sm">{&self.props.chapter}</span>
               </div>
               <button
                onclick={&self.props.on_refresh}
                class="z-50 mx-2 mb-2 text-white ">
                    <svg class="fill-current inline-block mb-1 my-auto self-center" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" >
                        <path class="heroicon-ui" d="M6 18.7V21a1 1 0 0 1-2 0v-5a1 1 0 0 1 1-1h5a1 1 0 1 1 0 2H7.1A7 7 0 0 0 19 12a1 1 0 1 1 2 0 9 9 0 0 1-15 6.7zM18 5.3V3a1 1 0 0 1 2 0v5a1 1 0 0 1-1 1h-5a1 1 0 0 1 0-2h2.9A7 7 0 0 0 5 12a1 1 0 1 1-2 0 9 9 0 0 1 15-6.7z"/>
                    </svg>
                </button>
            </div>
        }
    }
}
