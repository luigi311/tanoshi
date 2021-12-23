use rquickjs::{FromJs, IntoJs};
use serde::{Deserialize, Serialize};
/// A type represent chapter, normalized across source
///
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "js", derive(FromJs, IntoJs))]
#[cfg_attr(feature = "js", quickjs(rename_all = "camelCase"))]
pub struct ChapterInfo {
    pub source_id: i64,
    pub title: String,
    pub path: String,
    pub number: f64,
    pub scanlator: Option<String>,
    pub uploaded: i64,
}
