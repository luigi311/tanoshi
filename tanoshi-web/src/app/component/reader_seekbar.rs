use yew::prelude::*;
use web_sys::HtmlElement;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub visible: bool,
    pub disabled: bool,
    pub length: usize,
    pub current_page: usize,
    pub on_input: Callback<InputData>
}

pub struct ReaderSeekbar {
    props: Props,
    link: ComponentLink<Self>,
    root_ref: NodeRef,
}

pub enum Msg {
    Noop
}

impl Component for ReaderSeekbar{
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        ReaderSeekbar{
            props,
            link,
            root_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        false
    }

    fn change(&mut self, props: Self::Properties) -> bool {
       if self.props != props {
           self.props = props;
           if !self.props.visible {
               if let Some(bar) = self.root_ref.cast::<HtmlElement>() {
                   bar.class_list()
                       .remove_1("slideInUp")
                       .expect("failed remove class");
                   bar.class_list()
                       .add_1("slideOutDown")
                       .expect("failed add class");
               }
           } else {
               if let Some(bar) = self.root_ref.cast::<HtmlElement>() {
                   bar.class_list()
                       .remove_1("slideOutDown")
                       .expect("failed remove class");
                   bar.class_list()
                       .add_1("slideInUp")
                       .expect("failed add class");
               }
           }
           true
       } else {
           false
       }
    }

    fn view(&self) -> Html {
        html!{
            <div ref=self.root_ref.clone()
                 class="animated slideInUp faster block fixed inset-x-0 bottom-0 z-50 bg-gray-900 opacity-75 shadow safe-bottom">
                <div class="flex px-4 py-5 justify-center">
                    <span class="mx-4 text-white">{self.props.current_page + 1}</span>
                    <input
                        disabled={self.props.disabled}
                        type="range"
                        min="0"
                        max={self.props.length.checked_sub(1).unwrap_or(0)}
                        step="1"
                        value={self.props.current_page}
                        oninput={&self.props.on_input}/>
                    <span class="mx-4 text-white">{self.props.length}</span>
                </div>
            </div>
        }
    }
}