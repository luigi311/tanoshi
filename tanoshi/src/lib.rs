pub mod mangadex {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    #[derive(Deserialize, Serialize, Debug, Clone, Default)]
    pub struct MangadexLogin {
        pub login_username: String,
        pub login_password: String,
        pub remember_me: bool,
        pub two_factor: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct GetMangaResponse {
        pub manga: Manga,
        pub status: String,
    }
    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct GetChapterResponse {
        pub chapter: HashMap<String, Chapter>,
        pub status: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Manga {
        pub cover_url: String,
        pub description: String,
        pub title: String,
        pub artist: String,
        pub author: String,
        pub status: i64,
        pub genres: Vec<i64>,
        pub last_chapter: String,
        pub lang_name: String,
        pub lang_flag: String,
        pub hentai: i64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Chapter {
        pub volume: String,
        pub chapter: String,
        pub title: String,
        pub lang_code: String,
        pub timestamp: i64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct GetPagesResponse {
        pub id: i64,
        pub timestamp: i64,
        pub hash: String,
        pub volume: String,
        pub chapter: String,
        pub title: String,
        pub server: String,
        pub page_array: Vec<String>,
        pub status: String,
    }
}

pub mod manga {
    use chrono::Local;
    use human_sort::compare;
    use serde::{Deserialize, Serialize};
    use std::cmp::Ordering;

    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct Source {
        pub id: i32,
        pub name: String,
        pub url: String,
    }

    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct Manga {
        pub id: i32,
        pub title: String,
        pub author: String,
        //pub genre: Vec<String>,
        pub status: String,
        pub description: String,
        pub path: String,
        pub thumbnail_url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_read: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_page: Option<i32>,
        pub is_favorite: bool,
    }

    #[derive(Debug, Deserialize, Serialize, Clone, Eq, PartialEq, Ord)]
    pub struct Chapter {
        pub id: i32,
        pub manga_id: i32,
        pub no: String,
        pub title: String,
        pub url: String,
        pub read: i32,
        pub uploaded: chrono::NaiveDateTime,
    }

    impl Default for Chapter {
        fn default() -> Self {
            Chapter {
                id: 0,
                manga_id: 0,
                no: "".to_string(),
                title: "".to_string(),
                url: "".to_string(),
                read: 0,
                uploaded: chrono::NaiveDateTime::from_timestamp(0, 0),
            }
        }
    }

    impl PartialOrd for Chapter {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(compare(&self.no, &other.no))
        }
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum SortByParam {
        LastUpdated,
        Title,
        Comment,
        Views,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum SortOrderParam {
        Asc,
        Desc,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct Params {
        pub keyword: Option<String>,
        pub page: Option<String>,
        pub sort_by: Option<SortByParam>,
        pub sort_order: Option<SortOrderParam>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct GetParams {
        pub refresh: Option<bool>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct GetSourceResponse {
        pub sources: Vec<Source>,
        pub status: String,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct GetMangasResponse {
        pub mangas: Vec<Manga>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct GetMangaResponse {
        pub manga: Manga,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct GetChaptersResponse {
        pub chapters: Vec<Chapter>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct GetPagesResponse {
        pub manga_id: i32,
        pub pages: Vec<String>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct ImageProxyParam {
        pub url: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct HistoryRequest {
        pub chapter_id: i32,
        pub read: i32,
        pub at: chrono::NaiveDateTime,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct HistoryResponse {
        pub history: Vec<History>,
        pub status: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct History {
        pub manga_id: i32,
        pub title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub thumbnail_url: Option<String>,
        pub chapter: String,
        pub chapter_id: i32,
        pub read: i32,
        pub at: chrono::NaiveDateTime,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub days: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub show_sep: Option<bool>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct Update {
        pub manga_id: i32,
        pub title: String,
        pub thumbnail_url: String,
        pub number: String,
        pub chapter_id: i32,
        pub uploaded: chrono::NaiveDateTime,
        pub days: Option<i64>,
        pub show_sep: Option<bool>,
    }

    impl Default for Update {
        fn default() -> Self {
            Update {
                manga_id: 0,
                title: "".to_string(),
                thumbnail_url: "".to_string(),
                number: "".to_string(),
                chapter_id: 0,
                uploaded: Local::now().naive_local(),
                days: None,
                show_sep: None,
            }
        }
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct UpdatesResponse {
        pub updates: Vec<Update>,
        pub status: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct GetFavoritesResponse {
        pub favorites: Option<Vec<FavoriteManga>>,
        pub status: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct AddFavoritesResponse {
        pub status: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct FavoriteManga {
        pub manga_id: i32,
        pub title: String,
        pub path: String,
        pub thumbnail_url: String,
    }
}
