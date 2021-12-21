use rquickjs::FromJs;

#[derive(Debug, Clone, FromJs)]
#[quickjs(untagged)]
pub enum Lang {
    All,
    Single(String),
    Multi(Vec<String>),
}

/// A type represent source
#[derive(Debug, Clone, FromJs)]
#[quickjs(rename_all = "camelCase")]
pub struct SourceInfo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub version: String,
    pub icon: String,
    pub languages: Lang,
    pub nsfw: bool,
}
