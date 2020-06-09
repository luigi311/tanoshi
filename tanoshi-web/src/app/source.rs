use super::component::{Manga, Spinner};
use web_sys::HtmlElement;
use yew::prelude::*;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use yew::utils::{document, window};

use tanoshi_lib::manga::{
    GetMangasResponse, Manga as MangaModel, Params, SortByParam, SortOrderParam, SourceLogin,
};

use crate::app::job;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Properties)]
pub struct Props {
    pub source_id: Option<i32>,
}

pub struct Source {
    link: ComponentLink<Self>,
    source_id: i32,
    page: i32,
    mangas: Vec<MangaModel>,
    is_fetching: bool,
    closure: Closure<dyn Fn()>,
    keyword: String,
    worker: Box<dyn Bridge<job::Worker>>,
    is_login_page: bool,
    login: SourceLogin,
}

pub enum Msg {
    MangaReady(GetMangasResponse),
    ScrolledDown,
    KeywordChanged(InputData),
    Search(Event),
    SourceLogin,
    Submit(Event),
    UsernameChange(InputData),
    PasswordChange(InputData),
    RememberMeChange(InputData),
    TwoFactorChange(InputData),
    Noop,
}

impl Component for Source {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let tmp_link = link.clone();
        let closure = Closure::wrap(Box::new(move || {
            let current_scroll = window().scroll_y().expect("error get scroll y")
                + window().inner_height().unwrap().as_f64().unwrap();
            let height = document()
                .get_element_by_id("catalogue")
                .expect("should have catalogue")
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .offset_height() as f64;

            if current_scroll >= height {
                tmp_link.send_message(Msg::ScrolledDown);
            }
        }) as Box<dyn Fn()>);

        let worker_callback = link.callback(|msg| match msg {
            job::Response::MangasFetched(data) => Msg::MangaReady(data),
            job::Response::LoginPosted(data) => Msg::SourceLogin,
            _ => Msg::Noop,
        });
        let worker = job::Worker::bridge(worker_callback);

        Source {
            link,
            source_id: props.source_id.unwrap(),
            page: 1,
            mangas: vec![],
            is_fetching: true,
            closure,
            keyword: "".to_string(),
            worker,
            is_login_page: false,
            login: SourceLogin::default(),
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.source_id != props.clone().source_id.unwrap() {
            self.source_id = props.source_id.unwrap();
            return true;
        }
        return false;
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.fetch_mangas();
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(data) => {
                self.is_fetching = false;
                let mut mangas = data.mangas;
                if self.page == 1 {
                    self.mangas = mangas;
                } else {
                    self.mangas.append(&mut mangas);
                }
            }
            Msg::ScrolledDown => {
                if !self.is_fetching {
                    self.page += 1;
                    self.fetch_mangas();
                }
            }
            Msg::KeywordChanged(e) => {
                self.keyword = e.value;
            }
            Msg::Search(e) => {
                e.prevent_default();
                self.mangas.clear();
                self.page = 1;
                self.fetch_mangas();
            }
            Msg::SourceLogin => {
                self.is_login_page = !self.is_login_page;
                self.is_fetching = false;
            }
            Msg::Submit(e) => {
                e.prevent_default();
                self.login();
            }
            Msg::UsernameChange(e) => {
                self.login.username = e.value;
            }
            Msg::PasswordChange(e) => {
                self.login.password = e.value;
            }
            Msg::RememberMeChange(e) => {
                self.login.remember_me = Some(!self.login.remember_me.unwrap_or(false));
            }
            Msg::TwoFactorChange(e) => {
                self.login.two_factor = Some(e.value);
            }
            Msg::Noop => {}
        }
        true
    }

    fn view(&self) -> Html {
        match window().onscroll() {
            Some(_) => {}
            None => window().set_onscroll(Some(self.closure.as_ref().unchecked_ref())),
        }

        return html! {
            <div class="container mx-auto pb-20">
                <div class="w-full px-2 pb-2 flex justify-between block fixed inset-x-0 top-0 z-50 bg-tachiyomi-blue shadow" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" class="mx-2 self-center flex-none">
                        <path class="heroicon-ui" d="M4.06 13a8 8 0 0 0 5.18 6.51A18.5 18.5 0 0 1 8.02 13H4.06zm0-2h3.96a18.5 18.5 0 0 1 1.22-6.51A8 8 0 0 0 4.06 11zm15.88 0a8 8 0 0 0-5.18-6.51A18.5 18.5 0 0 1 15.98 11h3.96zm0 2h-3.96a18.5 18.5 0 0 1-1.22 6.51A8 8 0 0 0 19.94 13zm-9.92 0c.16 3.95 1.23 7 1.98 7s1.82-3.05 1.98-7h-3.96zm0-2h3.96c-.16-3.95-1.23-7-1.98-7s-1.82 3.05-1.98 7zM12 22a10 10 0 1 1 0-20 10 10 0 0 1 0 20z"/>
                    </svg>
                    <form class="mx-2 flex-grow" onsubmit=self.link.callback(|e| Msg::Search(e))>
                    <input type="search"
                            class="w-full px-3 py-2 focus:outline-none text-sm leading-tight text-white bg-tachiyomi-blue-darker shadow-inner rounded appearance-none"
                            placeholder={"Search"}
                            oninput=self.link.callback(|e| Msg::KeywordChanged(e))/>
                    </form>
                    <button onclick=self.link.callback(|_| Msg::SourceLogin)
                        class="hover:bg-tachiyomi-blue-darker focus:bg-tachiyomi-blue-darker rounded flex-none">
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" class="mx-2 my-auto self-center">
                            <path class="heroicon-ui" d="M19 10h2a1 1 0 0 1 0 2h-2v2a1 1 0 0 1-2 0v-2h-2a1 1 0 0 1 0-2h2V8a1 1 0 0 1 2 0v2zM9 12A5 5 0 1 1 9 2a5 5 0 0 1 0 10zm0-2a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm8 11a1 1 0 0 1-2 0v-2a3 3 0 0 0-3-3H7a3 3 0 0 0-3 3v2a1 1 0 0 1-2 0v-2a5 5 0 0 1 5-5h5a5 5 0 0 1 5 5v2z"/>
                        </svg>
                    </button>
                </div>
                {if !self.is_login_page{self.view_mangas()} else {self.view_login_page()}}
            </div>
        };
    }

    fn destroy(&mut self) {
        window().set_onscroll(None);
    }
}

impl Source {
    fn view_mangas(&self) -> Html {
        html! {
            <>
                <div class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2 pt-12" id="catalogue" style="margin-top: calc(env(safe-area-inset-top) + .5rem)">
                    { for self.mangas.iter().map(|manga| html!{
                        <Manga
                        id=manga.id
                        title=manga.title.to_owned()
                        thumbnail=manga.thumbnail_url.to_owned()
                        is_favorite=manga.is_favorite />
                    })
                    }
                </div>
                <div class="grid grid-cols-1" id="catalogue">
                {
                    match self.is_fetching {
                        true => html!{<Spinner is_active=true is_fullscreen=false />},
                        false => html!{<button onclick=self.link.callback(|_| Msg::ScrolledDown)>{"Load More"}</button>}
                    }
                }
                </div>
            </>
        }
    }

    fn view_login_page(&self) -> Html {
        html! {
            <div class="flex justify-center px-6 my-12" style="margin-top: calc(env(safe-area-inset-top) + .5rem)">
                <div class="w-full xl:w-3/4 lg:w-11/12 flex">
                    <div class="w-full  bg-white p-5 rounded-lg lg:rounded-l-none">
                        <form class="px-8 pt-6 pb-8 mb-4 bg-white rounded" onsubmit=self.link.callback(|e| Msg::Submit(e))>
                            <div class="mb-4">
                                <label class="block mb-2 text-sm font-bold text-gray-700" for="username">
                                    {"Username"}
                                </label>
                                <input
                                    class="w-full px-3 py-2 text-sm leading-tight text-gray-700 border rounded shadow appearance-none focus:outline-none focus:shadow-outline"
                                    id="username"
                                    type="text"
                                    value=self.login.username.to_owned()
                                    oninput=self.link.callback(|e| Msg::UsernameChange(e))
                                />
                            </div>
                            <div class="mb-4">
                                <label class="block mb-2 text-sm font-bold text-gray-700" for="password">
                                    {"Password"}
                                </label>
                                <input
                                    class="w-full px-3 py-2 mb-3 text-sm leading-tight text-gray-700 border rounded shadow appearance-none focus:outline-none focus:shadow-outline"
                                    id="password"
                                    type="password"
                                    value=self.login.password.clone()
                                    oninput=self.link.callback(|e| Msg::PasswordChange(e))
                                />
                                <label class="block mb-2 text-sm font-bold text-gray-700" for="remember-me">
                                    {"Remember Me"}
                                </label>
                                <input
                                    id="remember-me"
                                    type="checkbox"
                                    value=self.login.remember_me.unwrap_or(false).to_string()
                                    checked=self.login.remember_me.unwrap_or(false)
                                    oninput=self.link.callback(|e| Msg::RememberMeChange(e))
                                />
                                <label class="block mb-2 text-sm font-bold text-gray-700" for="token">
                                    {"2FA Code"}
                                </label>
                                <input
                                    class="w-full px-3 py-2 mb-3 text-sm leading-tight text-gray-700 border rounded shadow appearance-none focus:outline-none focus:shadow-outline"
                                    id="token"
                                    type="text"
                                    inputmode="numeric"
                                    value=self.login.two_factor.clone().unwrap_or("".to_string())
                                    oninput=self.link.callback(|e| Msg::TwoFactorChange(e))
                                />
                            </div>
                            <div class="mb-6 text-center">
                            {
                                match self.is_fetching {
                                    true => html!{<Spinner is_active=true is_fullscreen=false />},
                                    false => html!{
                                        <button
                                        class="w-full px-4 py-2 font-bold text-white bg-blue-500 rounded-full hover:bg-blue-700 focus:outline-none focus:shadow-outline"
                                        type="button">
                                        {"Sign In"}
                                    </button>
                                    }
                                }
                            }

                            </div>
                        </form>
                    </div>
                </div>
            </div>
        }
    }

    fn login(&mut self) {
        self.worker
            .send(job::Request::PostLogin(self.source_id, self.login.clone()));
        self.is_fetching = true;
    }

    fn fetch_mangas(&mut self) {
        self.worker.send(job::Request::FetchMangas(
            self.source_id,
            Params {
                keyword: Some(self.keyword.to_owned()),
                sort_by: Some(SortByParam::Views),
                sort_order: Some(SortOrderParam::Desc),
                page: Some(self.page.to_string()),
            },
        ));
        self.is_fetching = true;
    }
}
