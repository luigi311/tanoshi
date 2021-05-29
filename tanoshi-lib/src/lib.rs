#![crate_name = "tanoshi_lib"]

/// This is used to ensure both application and extension use the same version
pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Rust doesn't have stable ABI, this is used to ensure `rustc` version is match
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

/// This module contains model used in extensions and rest api
#[cfg(feature = "model")]
pub mod model {
    use core::f64;

    /// Model to login to source that require login, like mangadex to search
    #[derive(Debug, Clone, Default)]
    pub struct SourceLogin {
        pub username: String,
        pub password: String,
        pub remember_me: Option<bool>,
        pub two_factor: Option<String>,
    }

    /// Result of source login
    #[derive(Debug, Clone, Default)]
    pub struct SourceLoginResult {
        pub source_name: String,
        pub auth_type: String,
        pub value: String,
    }

    /// A type represent source
    #[derive(Debug, Clone)]
    pub struct Source {
        pub id: i64,
        pub name: String,
        pub url: String,
        pub version: String,
        pub icon: String,
        pub need_login: bool,
    }

    /// A type represent manga details, normalized across source
    #[derive(Debug, Clone, Default)]
    pub struct Manga {
        pub source_id: i64,
        pub title: String,
        pub author: Vec<String>,
        pub genre: Vec<String>,
        pub status: Option<String>,
        pub description: Option<String>,
        pub path: String,
        pub cover_url: String,
    }

    /// A type represent chapter, normalized across source
    #[derive(Debug, Clone)]
    pub struct Chapter {
        pub source_id: i64,
        pub title: String,
        pub path: String,
        pub number: f64,
        pub scanlator: String,
        pub uploaded: chrono::NaiveDateTime,
    }

    /// A type represent sort parameter for query manga from source, normalized across source
    #[derive(Debug, Clone, PartialEq)]
    pub enum SortByParam {
        LastUpdated,
        Title,
        Comment,
        Views,
    }

    impl Default for SortByParam {
        fn default() -> Self {
            SortByParam::Title
        }
    }

    /// A type represent order parameter for query manga from source, normalized across source
    #[derive(Debug, Clone, PartialEq)]
    pub enum SortOrderParam {
        Asc,
        Desc,
    }

    impl Default for SortOrderParam {
        fn default() -> Self {
            SortOrderParam::Asc
        }
    }
}

/// This module contains `Extension` trait, and function for interacting with `Extension`
#[cfg(feature = "extensions")]
pub mod extensions {
    use crate::model::{
        Chapter, Manga, SortByParam, SortOrderParam, Source, SourceLogin, SourceLoginResult
    };
    use anyhow::{anyhow, Result};
    use serde_yaml;
    use std::io::Read;

    /// `Extension` trait is an implementation for building extensions
    pub trait Extension: Send + Sync {
        /// Returns the information of the source
        fn detail(&self) -> Source;

        /// Returns list of manga from the source
        ///
        /// # Arguments
        ///
        /// * `param` - Parameter to filter manga from source
        /// * `keyword` - Keyword of manga title to search
        /// * `genres` - List of genres of manga to search
        /// * `page` - Number of page
        /// * `sort_by` - Sort results by SortByParam
        /// * `sort_order` - Sort ascending or descending
        /// * `auth` - If source need login to search, this param used to provide credentials
        fn get_mangas(
            &self,
            keyword: Option<String>,
            genres: Option<Vec<String>>,
            page: Option<i32>,
            sort_by: Option<SortByParam>,
            sort_order: Option<SortOrderParam>,
            auth: Option<String>,
        ) -> Result<Vec<Manga>>;

        /// Returns detail of manga
        fn get_manga_info(&self, path: &String) -> Result<Manga>;

        /// Returns list of chapters of a manga
        fn get_chapters(&self, path: &String) -> Result<Vec<Chapter>>;

        /// Returns list of pages from a chapter of a manga
        fn get_pages(&self, path: &String) -> Result<Vec<String>>;

        /// Proxy image
        fn get_page(&self, url: &String) -> Result<Vec<u8>> {
            let bytes = {
                let resp = ureq::get(url).call()?;
                let mut reader = resp.into_reader();
                let mut bytes = vec![];
                if reader.read_to_end(&mut bytes).is_err() {
                    return Err(anyhow!("error read image"));
                }
                bytes
            };
            Ok(bytes)
        }

        /// Login to source
        fn login(&self, _: SourceLogin) -> Result<SourceLoginResult> {
            Err(anyhow!("not implemented"))
        }
    }

    /// A type represents an extension
    pub struct PluginDeclaration {
        pub rustc_version: &'static str,
        pub core_version: &'static str,
        pub name: &'static str,
        pub register: unsafe fn(&mut dyn PluginRegistrar, Option<&serde_yaml::Value>),
    }

    /// A trait for register an extension
    pub trait PluginRegistrar {
        fn register_function(&mut self, name: &str, extension: Box<dyn Extension>);
    }

    /// macro for export an extension
    #[macro_export]
    macro_rules! export_plugin {
        ($name:expr, $register:expr) => {
            #[doc(hidden)]
            #[no_mangle]
            pub static plugin_declaration: $crate::extensions::PluginDeclaration =
                $crate::extensions::PluginDeclaration {
                    rustc_version: $crate::RUSTC_VERSION,
                    core_version: $crate::CORE_VERSION,
                    name: $name,
                    register: $register,
                };
        };
    }
}
