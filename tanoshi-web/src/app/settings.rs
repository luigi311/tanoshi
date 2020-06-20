use super::component::model::{
    BackgroundColor, Claims, PageRendering, ReadingDirection, SettingParams, User,
};
use serde::Deserialize;
use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::{html, ChangeData, Component, ComponentLink, Html, InputData, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use wasm_bindgen::prelude::*;
use web_sys::window;

use crate::app::AppRoute;

pub struct UserRow {
    pub user: User,
    pub is_edit: bool,
    pub is_new: bool,
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
    me_username: String,
    me_role: String,
    me_password: Option<String>,
    me_confirm_password: Option<String>,
    change_password: bool,
    closure: Closure<dyn FnMut(JsValue)>,
}

pub enum Msg {
    SetReadingDirection(ReadingDirection),
    SetBackgroundColor(BackgroundColor),
    SetPageRendering(PageRendering),
    Authorized(Claims),
    UserListReady(Vec<User>),
    NewUser,
    EditUser(usize),
    UsernameChange(usize, String),
    RoleChange(usize, ChangeData),
    ChangePassword,
    PasswordChange(InputData),
    ConfirmPasswordChange(InputData),
    SubmitPassword,
    PasswordChangedReady,
    SaveUser(usize),
    SaveUserSuccess(usize),
    ClearCache,
    Noop,
}

impl Component for Settings {
    type Message = Msg;
    type Properties = Props;

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let settings = {
            if let Ok(settings) = storage.restore("settings") {
                serde_json::from_str(settings.as_str()).expect("failed to serialize")
            } else {
                SettingParams::default()
            }
        };

        let token = storage
            .restore::<Result<String, _>>("token")
            .unwrap_or("".to_string());

        let closure = Closure::wrap(Box::new(move |value| {
            log::info!("cache {:?}", value);
        }) as Box<dyn FnMut(JsValue)>);

        Settings {
            fetch_task: None,
            link,
            storage,
            settings,
            token: token,
            is_admin: false,
            users: vec![],
            me_username: "".to_string(),
            me_role: "".to_string(),
            me_confirm_password: None,
            me_password: None,
            change_password: false,
            closure,
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
                self.me_username = claim.sub;
                self.me_role = claim.role;
                if self.is_admin {
                    self.fetch_users();
                }
            }
            Msg::UserListReady(users) => {
                self.users = users
                    .iter()
                    .map(|user| UserRow {
                        user: user.clone(),
                        is_edit: false,
                        is_new: false,
                    })
                    .collect();
            }
            Msg::NewUser => self.users.push(UserRow {
                user: User {
                    username: "New user".to_string(),
                    password: None,
                    role: "READER".to_string(),
                },
                is_edit: true,
                is_new: true,
            }),
            Msg::EditUser(i) => {
                self.users[i].is_edit = true;
            }
            Msg::UsernameChange(i, username) => {
                self.users[i].user.username = username;
            }
            Msg::RoleChange(i, e) => match e {
                ChangeData::Select(el) => {
                    self.users[i].user.role = el.value().clone();
                }
                _ => {}
            },
            Msg::ChangePassword => {
                self.change_password = !self.change_password;
            }
            Msg::PasswordChange(e) => {
                self.me_password = Some(e.value);
            }
            Msg::ConfirmPasswordChange(e) => {
                self.me_confirm_password = Some(e.value);
            }
            Msg::SubmitPassword => {
                self.change_password();
            }
            Msg::PasswordChangedReady => {
                self.me_password = None;
                self.me_confirm_password = None;
                self.change_password = false;
            }
            Msg::SaveUser(i) => {
                if self.users[i].is_new {
                    self.register_user(i);
                } else {
                    self.modify_user_role(i);
                }
            }
            Msg::SaveUserSuccess(i) => {
                self.users[i].is_edit = false;
                self.users[i].is_new = false;
            }
            Msg::ClearCache => {
                let _ = window()
                    .unwrap()
                    .caches()
                    .unwrap()
                    .delete("tanoshi")
                    .then(&self.closure);
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
            self.validate_token();
        }
    }

    fn view(&self) -> Html {
        html! {
            <div class="container pb-20" style="padding-top: calc(env(safe-area-inset-top) + .5rem)">
                {self.account_setting()}
                {
                    if self.is_admin {
                        self.admin_settings()
                    } else {
                        html!{}
                    }
                }
                {self.reading_settings()}
                {self.misc_settings()}
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
            self.link.callback(
                |response: Response<Json<Result<UserListResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(res))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::UserListReady(res.users);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }

    fn modify_user_role(&mut self, i: usize) {
        let req = Request::put("/api/user/role")
            .header("Authorization", self.token.clone())
            .header("Content-Type", "application/json")
            .body(Json(&self.users[i].user))
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(move |response: Response<Text>| {
                if let (meta, Ok(_res)) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::SaveUserSuccess(i);
                    }
                }
                Msg::Noop
            }),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }

    fn change_password(&mut self) {
        if self.me_password != self.me_confirm_password {
            return;
        }

        let req = Request::put("/api/user/password")
            .header("Authorization", self.token.clone())
            .header("Content-Type", "text/plain")
            .body(Ok(self.me_password.clone().unwrap()))
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(move |response: Response<Text>| {
                if let (meta, Ok(_res)) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::PasswordChangedReady;
                    }
                }
                Msg::Noop
            }),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }

    fn register_user(&mut self, i: usize) {
        let req = Request::post("/api/register")
            .header("Authorization", self.token.clone())
            .header("Content-Type", "application/json")
            .body(Json(&self.users[i].user))
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(move |response: Response<Text>| {
                if let (meta, Ok(_res)) = response.into_parts() {
                    if meta.status.is_success() {
                        return Msg::SaveUserSuccess(i);
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
            <div class={"shadow p-2 bg-tachiyomi-blue rounded-t"}>
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
            <div class="flex flex-col rounded-lg border border-grey-light m-2" id="admin">
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
                                        if !self.users[i].is_edit || !self.users[i].is_new {
                                           html!{self.users[i].user.username.clone()}
                                        } else {
                                            html!{
                                                <input
                                                    class="w-full border-b border-grey-light"
                                                    value=self.users[i].user.username.clone()
                                                    oninput=self.link.callback(move |e: InputData| Msg::UsernameChange(i, e.value))/>
                                            }
                                        }
                                    }</td>
                                    <td class="p-2">
                                    {
                                        if !self.users[i].is_edit {
                                            html!{self.users[i].user.role.clone()}
                                        } else {
                                            html!{
                                                <select onchange=self.link.callback(move |e: ChangeData| Msg::RoleChange(i, e))>
                                                    <option value="READER" selected={self.users[i].user.role.clone() == "READER".to_string()}>{"READER"}</option>
                                                    <option value="ADMIN" selected={self.users[i].user.role.clone() == "ADMIN".to_string()}>{"ADMIN"}</option>
                                                </select>
                                            }
                                        }
                                    }
                                    </td>
                                    <td class="p-2">
                                        <button
                                            class="bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold px-4 rounded"
                                            onclick={
                                                if !self.users[i].is_edit {
                                                    self.link.callback(move |_| Msg::EditUser(i))
                                                } else {
                                                    self.link.callback(move |_| Msg::SaveUser(i))
                                                }
                                            }>
                                            {if !self.users[i].is_edit {"Edit"} else {"Save"}}
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

    fn account_setting(&self) -> Html {
        html! {
            <div class="flex flex-col rounded-lg border border-grey-light m-2" id="account-setting">
                {self.separator("Account")}
                {self.setting_card("Username", html! {
                    <span>{self.me_username.clone()}</span>
                })}
                {self.setting_card("Role", html! {
                    <span>{self.me_role.clone()}</span>
                })}
                {
                    if self.change_password {
                        html!{
                            <>
                            {self.setting_card("New Password", html! {
                                <input
                                    class="w-full border-b border-grey-light"
                                    type="password"
                                    value=self.me_password.clone().unwrap_or("".to_string()).to_owned()
                                    oninput=self.link.callback(|e| Msg::PasswordChange(e))/>
                            })}
                            {self.setting_card("Confirm Password", html! {
                                <div class="flex flex-col">
                                <input
                                    class="w-full border-b border-grey-light"
                                    type="confirm-password"
                                    value=self.me_confirm_password.clone().unwrap_or("".to_string()).to_owned()
                                    oninput=self.link.callback(|e| Msg::ConfirmPasswordChange(e))/>
                                {
                                    if self.me_password != self.me_confirm_password {
                                        html!{<span class="text-xs text-red-500">{"Password doesn't match"}</span>}
                                    }
                                    else {
                                        html!{}
                                    }
                                }
                                </div>
                            })}
                            <button class={"bg-grey-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r border-b"}
                                onclick=self.link.callback(|_| Msg::SubmitPassword)>
                                {"Submit"}
                            </button>
                            </>
                        }
                    } else {
                        html!{}
                    }
                }
                <button class={"bg-grey-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r border-b"}
                    onclick=self.link.callback(|_| Msg::ChangePassword)>
                    {if !self.change_password {"Change Password"} else {"Cancel"}}
                </button>
                <RouterAnchor<AppRoute> route=AppRoute::Logout classes={"bg-grey-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-r text-center"}>
                    {"Logout"}
                </RouterAnchor<AppRoute>>
            </div>
        }
    }

    fn reading_settings(&self) -> Html {
        html! {
            <div class="flex flex-col rounded-lg border border-grey-light m-2" id="reading-setting">
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

    fn misc_settings(&self) -> Html {
        html! {
            <div class="flex flex-col rounded-lg border border-grey-light m-2" id="misc-setting">
                {self.separator("Miscellaneous")}
                {
                    self.setting_card("Clear Cache", html! {
                        <button class={"bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded-l"}
                            onclick=self.link.callback(|_| Msg::ClearCache)>
                            {"Clear"}
                        </button>
                    })
                }
            </div>
        }
    }
}
