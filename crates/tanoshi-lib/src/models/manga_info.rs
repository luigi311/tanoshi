use rquickjs::FromJs;

/// A type represent manga details, normalized across source
#[derive(Debug, FromJs)]
#[quickjs(rename_all = "camelCase")]
pub struct MangaInfo {
    pub source_id: i64,
    pub title: String,
    pub author: Vec<String>,
    pub genre: Vec<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub path: String,
    pub cover_url: String,
}
