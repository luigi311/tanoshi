use super::component::{Filter, Manga, MangaList, Spinner, WeakComponentLink};
use web_sys::HtmlElement;
use yew::prelude::*;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use yew::utils::{document, window};

use tanoshi_lib::manga::{Manga as MangaModel, Params, SortByParam, SortOrderParam, SourceLogin};
use tanoshi_lib::rest::GetMangasResponse;

use crate::app::job;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Properties)]
pub struct Props {
    pub source_name: String,
}

pub struct Source {
    link: ComponentLink<Self>,
    source_name: String,
    page: i32,
    mangas: Vec<MangaModel>,
    is_fetching: bool,
    closure: Closure<dyn Fn()>,
    keyword: String,
    worker: Box<dyn Bridge<job::Worker>>,
    is_login_page: bool,
    login: SourceLogin,
    show_filter: bool,
}

pub enum Msg {
    MangasReady(GetMangasResponse),
    ScrolledDown,
    KeywordChanged(InputData),
    Search(FocusEvent),
    SourceLogin,
    LoginSuccess,
    Submit(FocusEvent),
    UsernameChange(InputData),
    PasswordChange(InputData),
    RememberMeChange(InputData),
    TwoFactorChange(InputData),
    Filter,
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
            job::Response::MangasFetched(data) => Msg::MangasReady(data),
            job::Response::LoginPosted(_data) => Msg::LoginSuccess,
            _ => Msg::Noop,
        });
        let worker = job::Worker::bridge(worker_callback);

        Source {
            link,
            source_name: props.source_name,
            page: 1,
            mangas: vec![],
            is_fetching: true,
            closure,
            keyword: "".to_string(),
            worker,
            is_login_page: false,
            login: SourceLogin::default(),
            show_filter: false,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.source_name != props.clone().source_name {
            self.source_name = props.source_name;
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
            Msg::MangasReady(data) => {
                self.is_fetching = false;

                let mut mangas = data.mangas.clone();
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
            Msg::LoginSuccess => {
                self.is_login_page = false;
                self.fetch_mangas();
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
            Msg::RememberMeChange(_e) => {
                self.login.remember_me = Some(!self.login.remember_me.unwrap_or(false));
            }
            Msg::TwoFactorChange(e) => {
                self.login.two_factor = Some(e.value);
            }
            Msg::Filter => {
                self.show_filter = !self.show_filter;
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
                    <button onclick=self.link.callback(|_| Msg::Filter) class="hover:bg-tachiyomi-blue-darker focus:bg-tachiyomi-blue-darker rounded flex-none">
                        <svg fill="none" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" class="mx-2 self-center flex-none"><path d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"></path></svg>
                    </button>
                    <form class="mx-2 flex-grow" onsubmit=self.link.callback(|e| Msg::Search(e))>
                        <input type="search"
                            class="w-full px-3 py-2 focus:outline-none text-sm leading-tight text-white bg-tachiyomi-blue-darker shadow-inner rounded appearance-none"
                            placeholder={"Search"}
                            value={self.keyword.clone()}
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
                <Filter show={self.show_filter}/>
            </div>
        };
    }

    fn destroy(&mut self) {
        window().set_onscroll(None);
    }
}

impl Source {
    fn view_mangas(&self) -> Html {
        let list_link = &WeakComponentLink::<MangaList>::default();
        html! {
            <>
                <MangaList weak_link=list_link style="margin-top: calc(env(safe-area-inset-top) + .5rem)">
                    { for self.mangas.iter().map(|manga| {
                        html_nested!{
                        <Manga
                            key=manga.id
                            id=manga.id
                            title=&manga.title
                            thumbnail=&manga.thumbnail_url
                            is_favorite=&manga.is_favorite />
                    }})
                    }
                </MangaList>
                <div class="grid grid-cols-1" id="catalogue">
                {
                    match self.is_fetching {
                        true => html!{<Spinner is_active=true is_fullscreen=false />},
                        false => html!{<button class="flex rounded-lg border border-grey-light m-2 shadow justify-center" onclick=self.link.callback(|_| Msg::ScrolledDown)>{"Load More"}</button>}
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
                                        type="submit">
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
        self.worker.send(job::Request::PostLogin(
            self.source_name.clone(),
            self.login.clone(),
        ));
        self.is_fetching = true;
    }

    fn fetch_mangas(&mut self) {
        self.worker.send(job::Request::FetchMangas(
            self.source_name.clone(),
            Params {
                keyword: Some(self.keyword.to_owned()),
                sort_by: Some(SortByParam::Views),
                sort_order: Some(SortOrderParam::Desc),
                page: Some(self.page.to_string()),
                genres: None,
                refresh: match self.page {
                    1 => Some(true),
                    _ => None,
                },
            },
        ));
        self.is_fetching = true;
    }
}
