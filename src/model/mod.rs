use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manga {
    pub path: String,
    pub title: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chapter {
    pub path: String,
    pub read: i32,
    pub len: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Track {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct History {
    pub path: String,
    pub timestamp: u32,
}

impl PartialEq for Manga {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.title == other.title && self.path == other.path
    }
}
impl Eq for Manga {}
