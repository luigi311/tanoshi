use rquickjs::FromJs;

/// A type represent source
#[derive(Debug, Clone, FromJs)]
#[quickjs(rename_all = "camelCase")]
pub struct SourceInfo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub version: String,
    pub icon: String,
    pub languages: String,
    pub nsfw: bool,
}
