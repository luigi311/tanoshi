use super::component::model::{
    BackgroundColor, Claims, PageRendering, ReadingDirection, SettingParams, User,
};
use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use serde::Deserialize;

pub struct UserRow {
    pub user: User,
    pub is_edit: bool,
}

#[derive(Deserialize)]
pub struct UserListResponse {
    users: Vec<User>,
}

#[derive(Clone, Properties)]
pub struct Props {}

pub struct Settings {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    storage: StorageService,
    settings: SettingParams,
    token: String,
    is_admin: bool,
    users: Vec<UserRow>,
}

pub enum Msg {
    SetReadingDirection(ReadingDirection),
    SetBackgroundColor(BackgroundColor),
    SetPageRendering(PageRendering),
    Authorized(Claims),
    UserListReady(Vec<User>),
    NewUser,
    EditUser(usize),
    Noop,
}

impl Component for Settings {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let settings = {
            if let Ok(settings) = storage.restore("settings") {
                serde_json::from_str(settings.as_str()).expect("failed to serialize")
            } else {
                SettingParams::default()
            }
        };

        Settings {
            fetch_task: None,
            link,
            storage,
            settings,
            token: "".to_string(),
            is_admin: false,
            users: vec![],
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetReadingDirection(value) => {
                self.settings.reading_direction = value;
                self.storage.store("settings", &self.settings)
            }
            Msg::SetBackgroundColor(value) => {
                self.settings.background_color = value;
                self.storage.store("settings", &self.settings)
            }
            Msg::SetPageRendering(value) => {
                self.settings.page_rendering = value;
                self.storage.store("settings", &self.settings)
            }
            Msg::Authorized(claim) => {
                self.is_admin = claim.role == "ADMIN".to_string();
                if self.is_admin {
                    self.fetch_users();
                }
            }
            Msg::UserListReady(users) => {
                self.users = users.iter().map(|user| UserRow{
                    user: user.clone(), 
                    is_edit: false
                }).collect();
            }
            Msg::NewUser => {
                self.users.push(UserRow{
                    user: User{
                        username: "New user".to_string(),
                        password: None,
                        role: "READER".to_string(),
                    },
                    is_edit: true,
                })
            }
            Msg::EditUser(i) => {
                self.users[i].is_edit = true;
            }
            Msg::Noop => {
                return false;
            }
        }
        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            if let Ok(token) = self.storage.restore("token") {
                self.token = token;
                self.validate_token();
            }
        }
    }

    fn view(&self) -> Html {
        html! {
            <div class="container pb-20" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                {
                    if self.is_admin {
                        self.admin_settings()
                    } else {
                        html!{}
                    }
                }
                {self.reading_settings()}
            </div>
        }
    }
}

impl Settings {
    fn validate_token(&mut self) {
        let req = Request::get("/api/validate")
            .header("Authorization", self.token.clone())
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link
                .callback(|response: Response<Json<Result<Claims, anyhow::Error>>>| {
                    if let (meta, Json(Ok(res))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::Authorized(res);
                        }
                    }
                    Msg::Noop
                }),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
    
    fn fetch_users(&mut self) {
        let req = Request::get("/api/user")
            .header("Authorization", self.token.clone())
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link
                .callback(|response: Response<Json<Result<UserListResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(res))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::UserListReady(res.users);
                        }
                    }
                    Msg::Noop
                }),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }

    fn separator(&self, text: &str) -> Html {
        html! {
            <div class={"shadow p-2 bg-tachiyomi-blue"}>
                <span class="text-semibold text-white">{text}</span>
            </div>
        }
    }

    fn setting_card(&self, label: &str, child: Html) -> Html {
        html! {
            <div class="flex justify-between border-b border-gray-light p-2 content-center">
                <span class="font-semibold my-auto">{label}</span>
                <div class="inline-flex">{child}</div>
            </div>
        }
    }

    fn admin_settings(&self) -> Html {
        html! {            
            <div class="flex flex-col rounded-lg border border-grey-light m-2" id="updates">
                {self.separator("Users")}
                {
                    html! {
                        <table class="table-fixed w-full text-left">
                            <thead class="border-b">
                                <tr>
                                    <th class="p-2">{"Username"}</th>
                                    <th class="p-2">{"Role"}</th>
                                    <th class="p-2">{"Actions"}</th>
                                </tr>
                            </thead>
                            <tbody>
                            {
                            for (0..self.users.len()).map(|i| html!{            
                                <tr class="border-b">
                                    <td class="p-2">{
                                        if !self.users[i].is_edit {
                                           html!{self.users[i].user.username.clone()}
                                        } else {
                                            html!{
                                                <input 
                                                    class="w-full border-b border-grey-light"
                                                    value=self.users[i].user.username.clone()/>
                                            }
                                        }
                                    }</td>
                                    <td class="p-2">{self.users[i].user.role.clone()}</td>
                                    <td class="p-2"> 
                                        <button 
                                            class="bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold px-4 rounded"
                                            onclick=self.link.callback(move |_| Msg::EditUser(i))>
                                            {if !self.users[i].is_edit{"Edit"} else {"Save"}}
                                        </button>
                                    </td>
                                </tr>
                            })
                            }
                            </tbody>
                        </table>
                    }
                } 
                <button class={"bg-grey-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r"}
                    onclick=self.link.callback(|_| Msg::NewUser)>
                    {"New User"}
                </button>
            </div>
        }
    }

    fn reading_settings(&self) -> Html {
        html! {
            <div class="flex flex-col rounded-lg border border-grey-light m-2" id="updates">
                {self.separator("Reader")}
                {
                    self.setting_card("Direction", html! {
                        <>
                            <button class={
                                format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-l",
                                if self.settings.reading_direction == ReadingDirection::RightToLeft { "bg-gray-400" } else {"bg-gray-300"})}
                                onclick=self.link.callback(|_| Msg::SetReadingDirection(ReadingDirection::RightToLeft))>
                                {"Right to Left"}
                            </button>
                            <button class={
                                format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r",
                                if self.settings.reading_direction == ReadingDirection::LeftToRight { "bg-gray-400" } else {"bg-gray-300"})}
                                onclick=self.link.callback(|_| Msg::SetReadingDirection(ReadingDirection::LeftToRight))>
                                {"Left to Right"}
                            </button>
                        </>
                    })
                }
                {
                    self.setting_card("Background", html! {
                        <>
                            <button class={
                                format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-l",
                                if self.settings.background_color == BackgroundColor::White { "bg-gray-400" } else {"bg-gray-300"})}
                                onclick=self.link.callback(|_| Msg::SetBackgroundColor(BackgroundColor::White))>
                                {"White"}
                            </button>
                            <button class={
                                format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r",
                                if self.settings.background_color == BackgroundColor::Black { "bg-gray-400" } else {"bg-gray-300"})}
                                onclick=self.link.callback(|_| Msg::SetBackgroundColor(BackgroundColor::Black))>
                                {"Black"}
                            </button>
                        </>
                    })
                }
                {
                    self.setting_card("Mode", html! {
                        <>
                            <button class={
                                format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-l",
                                if self.settings.page_rendering == PageRendering::SinglePage { "bg-gray-400" } else {"bg-gray-300"})}
                                onclick=self.link.callback(|_| Msg::SetPageRendering(PageRendering::SinglePage))>
                                {"Single"}
                            </button>
                            <button class={
                                format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4",
                                if self.settings.page_rendering == PageRendering::DoublePage { "bg-gray-400" } else {"bg-gray-300"})}
                                onclick=self.link.callback(|_| Msg::SetPageRendering(PageRendering::DoublePage))>
                                {"Double"}
                            </button>
                            <button class={
                                 format!("{} hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r",
                                 if self.settings.page_rendering == PageRendering::LongStrip { "bg-gray-400" } else {"bg-gray-300"})}
                                 onclick=self.link.callback(|_| Msg::SetPageRendering(PageRendering::LongStrip))>
                                 {"Webtoon"}
                             </button>
                        </>
                    })
                }
            </div>
        }
    }
}
