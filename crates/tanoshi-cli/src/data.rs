#[derive(serde::Serialize)]
pub struct Index {
    pub path: String,
    pub id: i64,
    pub name: String,
    pub version: String,
    pub lib_version: String,
    pub icon: String,
}
