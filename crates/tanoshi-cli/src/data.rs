use tanoshi_lib::prelude::Source;

#[derive(serde::Serialize)]
pub struct Index {
    pub path: String,
    #[serde(flatten)]
    pub source: Source,
}
