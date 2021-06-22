use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A type represent source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub version: String,
    pub icon: String,
    pub need_login: bool,
}

/// A type represent manga details, normalized across source
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manga {
    pub source_id: i64,
    pub title: String,
    pub author: Vec<String>,
    pub genre: Vec<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub path: String,
    pub cover_url: String,
}

/// A type represent chapter, normalized across source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub source_id: i64,
    pub title: String,
    pub path: String,
    pub number: f64,
    pub scanlator: String,
    pub uploaded: chrono::NaiveDateTime,
}

/// Model to login to source that require login, like mangadex to search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceLogin {
    pub username: String,
    pub password: String,
    pub remember_me: Option<bool>,
    pub two_factor: Option<String>,
}

/// Result of source login
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceLoginResult {
    pub source_name: String,
    pub auth_type: String,
    pub value: String,
}

/// A type represent sort parameter for query manga from source, normalized across source
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortByParam {
    LastUpdated,
    Title,
    Comment,
    Views,
}

impl Default for SortByParam {
    fn default() -> Self {
        SortByParam::Title
    }
}

/// A type represent order parameter for query manga from source, normalized across source
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortOrderParam {
    Asc,
    Desc,
}

impl Default for SortOrderParam {
    fn default() -> Self {
        SortOrderParam::Asc
    }
}

pub type Headers = HashMap<String, Vec<String>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub keyword: Option<String>,
    pub genres: Option<Vec<String>>,
    pub page: Option<i32>,
    pub sort_by: Option<SortByParam>,
    pub sort_order: Option<SortOrderParam>,
    pub auth: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Option<Headers>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub headers: Headers,
    pub body: String,
    pub status: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionResult<T> {
    pub data: Option<T>,
    pub error: Option<String>
}

impl<T> ExtensionResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            data: Some(data),
            error: None
        }
    }
    
    pub fn err(msg: &str) -> Self {
        Self {
            data: None,
            error: Some(msg.to_string())
        }
    }
}