use serde::{Deserialize, Serialize};
pub mod settings;

#[derive(Deserialize, Serialize, Clone)]
pub struct SettingParams {
    pub double_page: bool,
    pub background_color: bool,
    pub direction: i32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct GetSettingsResponse {
    settings: Option<SettingParams>,
    status: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct SetSettingsResponse {
    status: String,
}
