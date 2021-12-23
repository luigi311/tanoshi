use rquickjs::{FromJs, IntoJs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "js", derive(FromJs, IntoJs))]
#[cfg_attr(feature = "js", quickjs(untagged))]
pub enum Lang {
    All,
    Single(String),
    Multi(Vec<String>),
}

/// A type represent source
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "js", derive(FromJs, IntoJs))]
#[cfg_attr(feature = "js", quickjs(rename_all = "camelCase"))]
pub struct SourceInfo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub version: String,
    pub icon: String,
    pub languages: Lang,
    pub nsfw: bool,
}
