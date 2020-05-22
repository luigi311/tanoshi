pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

#[cfg(feature = "model")]
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
        pub version: &str,
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

#[cfg(feature = "extensions")]
pub mod extensions {
    use crate::manga::{Chapter, Manga, Params, Source};
    use anyhow::{Result, anyhow};
    use std::io::Read;

    pub trait Extension: Send + Sync {
        fn info(&self) -> Source;
        fn get_mangas(
            &self,
            url: &String,
            param: Params,
            cookies: Vec<String>,
        ) -> Result<Vec<Manga>>;
        fn get_manga_info(&self, url: &String) -> Result<Manga>;
        fn get_chapters(&self, url: &String) -> Result<Vec<Chapter>>;
        fn get_pages(&self, url: &String) -> Result<Vec<String>>;
        fn get_page(&self, url: &String, bytes: &mut Vec<u8>) -> Result<String> {
            let resp = ureq::get(&url).call();
            let content_type = resp.content_type().to_owned();
            let mut reader = resp.into_reader();
            if reader.read_to_end(bytes).is_err() {
                return Err(anyhow!("error write image"));
            }

            Ok(content_type)
        }
    }

    pub struct PluginDeclaration {
        pub rustc_version: &'static str,
        pub core_version: &'static str,
        pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
    }

    pub trait PluginRegistrar {
        fn register_function(&mut self, name: &str, extension: Box<dyn Extension>);
    }

    #[macro_export]
    macro_rules! export_plugin {
        ($register:expr) => {
            #[doc(hidden)]
            #[no_mangle]
            pub static plugin_declaration: $crate::extensions::PluginDeclaration =
                $crate::extensions::PluginDeclaration {
                    rustc_version: $crate::RUSTC_VERSION,
                    core_version: $crate::CORE_VERSION,
                    register: $register,
                };
        };
    }
}
