use serde::{Deserialize, Serialize};
use yew::format::Text;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub username: String,
    pub password: Option<String>,
    pub role: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: i64,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum ReadingDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum PageRendering {
    SinglePage,
    DoublePage,
    LongStrip,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum BackgroundColor {
    Black,
    White,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct SettingParams {
    pub page_rendering: PageRendering,
    pub background_color: BackgroundColor,
    pub reading_direction: ReadingDirection,
}

impl Default for SettingParams {
    fn default() -> Self {
        SettingParams {
            page_rendering: PageRendering::SinglePage,
            background_color: BackgroundColor::Black,
            reading_direction: ReadingDirection::LeftToRight,
        }
    }
}

impl From<&SettingParams> for Text {
    fn from(param: &SettingParams) -> Self {
        let val = serde_json::to_string(&param).unwrap();
        Text::Ok(val)
    }
}
