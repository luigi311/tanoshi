use human_sort::compare;
use serde::{Deserialize, Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub mod mangasee;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manga {
    pub title: String,
    pub author: String,
    pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub url: String,
    pub thumbnail_url: String,
    pub chapter: BTreeMap<ChapterNumber, Chapter>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Chapter {
    pub prev_chapter: String,
    pub chapter: String,
    pub next_chapter: String,
    pub url: String,
    pub pages: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Params {
    pub keyword: Option<String>,
    pub page: Option<String>,
    pub sort_order: Option<String>,
    pub sort_by: Option<String>,
}

#[derive(Eq, Debug, Deserialize, Clone)]
pub struct ChapterNumber {
    number: String,
}

impl Serialize for ChapterNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.number)
    }
}

impl Ord for ChapterNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        compare(&self.number, &other.number)
    }
}

impl PartialOrd for ChapterNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ChapterNumber {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
}

pub trait Scraping {
    fn new(url: &'static str) -> Self;
    fn get_mangas(&self, param: Params) -> Vec<Manga>;
    fn get_latest_mangas(&self) -> Vec<Manga>;
    fn get_manga_info(&self, manga: &Manga) -> Manga;
    fn get_chapter(&self, chapter: &mut Chapter);
}
