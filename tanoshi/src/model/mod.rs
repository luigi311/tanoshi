use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Manga {
    pub path: String,
    pub title: String,
    pub source: String,
    pub thumbnail_url: String,
}

impl PartialEq for Manga {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.path == other.path
    }
}
impl Eq for Manga {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Chapter {
    pub path: String,
    pub read: i32,
}

impl PartialEq for Chapter {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for Chapter {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Track {
    pub title: String,
    pub url: String,
}

impl PartialEq for Track {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for Track {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct History {
    pub path: String,
    pub timestamp: u32,
}

impl PartialEq for History {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for History {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Document {
    pub manga: Manga,
    pub chapters: Vec<Chapter>,
    pub tracks: Vec<Track>,
    pub history: Vec<History>,
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.manga == other.manga
    }
}
impl Eq for Document {}
