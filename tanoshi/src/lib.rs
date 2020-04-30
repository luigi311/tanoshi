use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MangaModel {
    pub title: String,
    pub author: String,
    //pub genre: Vec<String>,
    pub status: String,
    pub description: String,
    pub path: String,
    pub thumbnail_url: String,
    pub last_read: Option<String>,
    pub last_page: Option<i32>,
    pub is_favorite: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetMangasResponse {
    pub mangas: Vec<MangaModel>,
}

pub mod mangadex {
    use serde::{Deserialize, Serialize};
    #[derive(Deserialize, Serialize, Debug, Clone, Default)]
    pub struct MangadexLogin {
        pub login_username: String,
        pub login_password: String,
        pub remember_me: bool,
        pub two_factor: String,
    }
}

pub mod manga {
    use serde::{Deserialize, Serialize};
    use chrono::Local;
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct Manga {
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

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct Chapter {
        pub no: String,
        pub url: String,
        pub read: i32,
        pub uploaded: chrono::NaiveDateTime,
    }

    impl Default for Chapter {
        fn default() -> Self {
            Chapter {
                no: "".to_string(),
                url: "".to_string(),
                read: 0,
                uploaded: Local::now().naive_local(),
            }
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
        pub pages: Vec<String>,
    }
}
