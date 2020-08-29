use yew::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlElement, Event};

use crate::app::component::{WeakComponentLink, Page};
use crate::app::reader::{Reader, Msg as ReaderMsg};
use crate::app::component::model::{PageRendering, ReadingDirection};

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub current_page: usize,
    pub pages: Vec<String>,
    pub reader_link: WeakComponentLink<Reader>,
    #[prop_or_default]
    pub on_page_change: Callback<usize>,
}

pub struct Webtoon {
    props: Props,
    link: ComponentLink<Self>,
    closure: Closure<dyn Fn()>,
    loaded_page: Vec<String>,
    page_scroll_height: Vec<i32>,
    scrolled: bool,
    count: i32,
}

pub enum Msg {
    ScrollEvent(i32),
    ImageLoad(usize),
    ImageError(usize, Event),
    Noop,
}

impl Component for Webtoon {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let tmp_link = link.clone();
        let closure = Closure::wrap(Box::new(move || {
            let current_scroll = window().unwrap().scroll_y().expect("error get scroll y")
                + window().unwrap().inner_height().unwrap().as_f64().unwrap();

            tmp_link.send_message(Msg::ScrollEvent(current_scroll as i32));
        }) as Box<dyn Fn()>);

        window().unwrap().set_onscroll(Some(closure.as_ref().unchecked_ref()));

        Webtoon {
            props,
            link,
            closure,
            loaded_page: vec![],
            page_scroll_height: vec![],
            scrolled: false,
            count: 0,
        }
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        match msg {
            Msg::ScrollEvent(scroll) => {
                log::info!("count: {}", self.count);
                self.count+= 1;
                let mut page = 0;
                for i in 0..self.props.pages.len() {
                    // log::debug!("current {} page {} scroll {} scroll top {}", self.props.current_page, page, scroll - el.scroll_height(), page as i32 *  el.scroll_height());
                    if scroll - self.page_scroll_height[i] > i as i32 * self.page_scroll_height[i] {
                        // log::debug!("should be last page {}", page);
                        page = i;
                    } else {
                        break;
                    }
                }
                if self.props.current_page != page {
                    self.props.current_page = page;
                    self.props.on_page_change.emit(self.props.current_page);
                    return true;
                } else {
                    return false;
                }
            }
            Msg::ImageLoad(index) => {
                if let Some(img) = window().unwrap().document().unwrap().get_element_by_id(format!("{}", index).as_str()) {
                    img.class_list().remove_4("border", "border-dashed", "md:h-screen", "h-page").expect("failed remove class");
                    img.class_list().add_1("h-auto").expect("failed add class");
                    img.remove_attribute("style").expect("failed remove attribute");
                    self.loaded_page[index] = img.get_attribute("src").unwrap();
                }
            }
            Msg::ImageError(index, event) => {
                if let Some(img) = window().unwrap().document().unwrap().get_element_by_id(format!("{}", index).as_str()) {
                    img.set_attribute("style", r#"background: transparent url("data:image/svg+xml;utf8,<svg fill="none" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" viewBox="0 0 24 24" stroke="currentColor"><path d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>") no-repeat scroll center center"#).expect("failed set property");
                }
            }
            Msg::Noop => {return false;}
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> bool {
        if self.props.current_page != props.current_page {
            self.props = props;
            true
        } else if self.props.pages != props.pages {
            self.props = props;
            self.loaded_page.clear();
            self.page_scroll_height.clear();
            for _ in self.props.pages.iter() {
                self.loaded_page.push("".to_string());
                self.page_scroll_height.push(0);
            }
            true
        } else {
            false
        }
    }

    fn rendered(&mut self, _first_render: bool) {
        if !self.scrolled {
            // log::debug!("first render");
            if let Some(el) = window().unwrap().document().unwrap().get_element_by_id(format!("{}", self.props.current_page).as_str()) {
                el.scroll_into_view();
                // log::debug!("scrolled");
                self.scrolled = true;
            }
        }
        log::info!("rendered");
        for page in 0..self.props.pages.len() {
            if let Some(el) = window().unwrap().document().unwrap().get_element_by_id(format!("{}", page).as_str()) {
                self.page_scroll_height[page] = el.scroll_height();
            }
        }
    }

    fn view(&self) -> Html {
        let reader_link = &self.props.reader_link.borrow().clone().unwrap();
        html! {
            <div class="h-screen m-0 outline-none" id="manga-reader" tabindex="0">
            <div class="flex flex-col justify-center overflow-auto">
                <div
                    class="border-dashed border-b border-gray-500 flex justify-center items-center h-24 cursor-pointer"
                    onmouseup={reader_link.callback(|_| ReaderMsg::PrevChapter)}>
                    <span class="text-gray-500">{"Previous Chapter"}</span>
                </div>
                    {
                        for self.props.pages
                            .clone()
                            .into_iter()
                            .enumerate()
                            .map(|(i, page)| {
                                html! {
                                     <img id={i}
                                        onload={self.link.callback(move |_| Msg::ImageLoad(i))}
                                        onerror={if self.page_or_empty(i, &page) != "" {self.link.callback(move |e| Msg::ImageError(i, e))} else {{self.link.callback(move |_| Msg::Noop)}}}
                                        class="block object-contain border border-dashed w-auto h-page md:h-screen cursor-pointer"
                                        src={self.page_or_empty(i, &page)}
                                        onmouseup={reader_link.callback(|_| ReaderMsg::ToggleBar)}
                                        style={"background: transparent url(\"data:image/gif;base64,R0lGODlhEAALAPQAAP///1t0m+fr8eHl7fL09194nlt0m3mOra+7zpmpwdPa5HGHqYuduLS/0ZyrwtXc5nSJql53nY6fuu/x9ebq8Pj5+4CTsens8ff4+tDX4sLL2t3i6vT2+Ft0m1t0m1t0myH5BAkLAAAAIf4aQ3JlYXRlZCB3aXRoIGFqYXhsb2FkLmluZm8AIf8LTkVUU0NBUEUyLjADAQAAACwAAAAAEAALAAAFLSAgjmRpnqSgCuLKAq5AEIM4zDVw03ve27ifDgfkEYe04kDIDC5zrtYKRa2WQgAh+QQJCwAAACwAAAAAEAALAAAFJGBhGAVgnqhpHIeRvsDawqns0qeN5+y967tYLyicBYE7EYkYAgAh+QQJCwAAACwAAAAAEAALAAAFNiAgjothLOOIJAkiGgxjpGKiKMkbz7SN6zIawJcDwIK9W/HISxGBzdHTuBNOmcJVCyoUlk7CEAAh+QQJCwAAACwAAAAAEAALAAAFNSAgjqQIRRFUAo3jNGIkSdHqPI8Tz3V55zuaDacDyIQ+YrBH+hWPzJFzOQQaeavWi7oqnVIhACH5BAkLAAAALAAAAAAQAAsAAAUyICCOZGme1rJY5kRRk7hI0mJSVUXJtF3iOl7tltsBZsNfUegjAY3I5sgFY55KqdX1GgIAIfkECQsAAAAsAAAAABAACwAABTcgII5kaZ4kcV2EqLJipmnZhWGXaOOitm2aXQ4g7P2Ct2ER4AMul00kj5g0Al8tADY2y6C+4FIIACH5BAkLAAAALAAAAAAQAAsAAAUvICCOZGme5ERRk6iy7qpyHCVStA3gNa/7txxwlwv2isSacYUc+l4tADQGQ1mvpBAAIfkECQsAAAAsAAAAABAACwAABS8gII5kaZ7kRFGTqLLuqnIcJVK0DeA1r/u3HHCXC/aKxJpxhRz6Xi0ANAZDWa+kEAA7\") no-repeat scroll center center"}
                                    />
                                }
                            })
                    }
                <div
                    class="border-dashed border-t border-gray-500 flex justify-center items-center h-24 cursor-pointer"
                    onmouseup={reader_link.callback(|_| ReaderMsg::NextChapter)}>
                    <span class="text-gray-500">{"Next Chapter"}</span>
                </div>
            </div>
            </div>
        }
    }

    fn destroy(&mut self) {
        window().unwrap().set_onscroll(None);
    }
}

impl Webtoon {
    fn page_or_empty(&self, i: usize, page: &String) -> String {
        if i >= self.props.current_page.checked_sub(1).unwrap_or(0) && i < self.props.current_page + 3
        {
            page.to_string()
        } else {
            self.loaded_page[i].clone()
        }
    }
}