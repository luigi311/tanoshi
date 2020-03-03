use serde::{Deserialize, Serialize};
pub mod settings;

#[derive(Deserialize, Serialize, Clone)]
pub enum ReadingDirection {
    LeftToRight,
    RightToLeft,
    LongStrip,
}

#[derive(Deserialize, Serialize, Clone)]
pub enum BackgroundColor {
    Black,
    White,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct SettingParams {
    pub double_page: bool,
    pub background_color: BackgroundColor,
    pub reading_direction: ReadingDirection,
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
