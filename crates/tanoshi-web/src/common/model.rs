use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub icon: String,
    pub has_update: bool,
    pub installed: bool,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
    pub telegram_chat_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerStatus {
    pub activated: bool,
    pub version: String,
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self {
            activated: false,
            version: "0.0.0".to_string(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DownloadQueue {
    pub source_name: String,
    pub manga_title: String,
    pub chapter_id: i64,
    pub chapter_title: String,
    pub downloaded: i64,
    pub total: i64,
    pub priority: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InputType {
    String(String),
    Number(f64),
    Boolean(bool),
}

impl From<String> for InputType {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for InputType {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<f64> for InputType {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for InputType {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Input {
    Text {
        name: String,
        state: Option<String>,
    },
    Checkbox {
        name: String,
        state: Option<bool>,
    },
    Select {
        name: String,
        values: Vec<InputType>,
        state: Option<i64>,
    },
    Group {
        name: String,
        state: Option<Vec<InputType>>,
    },
    Sort {
        name: String,
        values: Vec<InputType>,
        selection: Option<(i64, bool)>,
    },
}
