use yew::{Component, ComponentLink, html, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;
use crate::app::AppRoute;


#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub title: String,
    pub thumbnail: String,
    pub path: String,
    pub source: String,
    pub is_favorite: bool,
}

pub struct Manga {
    title: String,
    thumbnail: String,
    path: String,
    pub source: String,
    pub is_favorite: bool,
}

pub enum Msg {

}

impl Component for Manga {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Manga {
            title: props.title,
            thumbnail: props.thumbnail,
            path: props.path,
            source: props.source,
            is_favorite: props.is_favorite,
        }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let _title = self.title.to_owned();
        let thumbnail = self.thumbnail.to_owned();
        let path = self.path.to_owned();
        let source = self.source.to_owned();

        html! {
                <div class="col">
                    <div class={if self.is_favorite {"manga-cover-container favorite"} else {"manga-cover-container"}}>
                        <RouterAnchor<AppRoute> route=AppRoute::Detail(source, path.replace("/manga/", ""))>
                        { html!{
                            <div>
                            <img class="manga-cover" src=thumbnail/>
                            </div>
                        }}
                        </RouterAnchor<AppRoute>>
                    </div>
                </div>
        }
    }
}