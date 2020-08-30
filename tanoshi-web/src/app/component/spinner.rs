use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub is_active: bool,
    pub is_fullscreen: bool,
}

pub struct Spinner {
    is_active: bool,
    is_fullscreen: bool,
}

pub enum Msg {}

impl Component for Spinner {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Spinner {
            is_active: props.is_active,
            is_fullscreen: props.is_fullscreen,
        }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.is_active != props.is_active {
            self.is_active = props.is_active;
            return true;
        }
        false
    }
    //w-full h-full block m-auto bg-white opacity-75 z-10
    fn view(&self) -> Html {
        let class =
            if !self.is_active {"hidden"}
            else if self.is_fullscreen{"w-full h-full fixed flex top-0 left-0 bg-white dark:bg-black bg-opacity-50 z-10 justify-center"}
            else {"w-full h-full block flex m-auto z-10 justify-center"};
        html! {
                <div class={class}>
                    <div class="loader-dots block relative w-20 h-5 m-auto">
                        <div class="absolute top-0 mt-1 w-3 h-3 rounded-full bg-tachiyomi-blue"></div>
                        <div class="absolute top-0 mt-1 w-3 h-3 rounded-full bg-tachiyomi-blue"></div>
                        <div class="absolute top-0 mt-1 w-3 h-3 rounded-full bg-tachiyomi-blue"></div>
                        <div class="absolute top-0 mt-1 w-3 h-3 rounded-full bg-tachiyomi-blue"></div>
                    </div>
                </div>
        }
    }
}