use std::collections::HashMap;

use async_trait::async_trait;

use crate::models::{ChapterInfo, Input, MangaInfo, SourceInfo};
use anyhow::Result;

#[async_trait]
pub trait Extension: Send + Sync {
    fn get_source_info(&self) -> SourceInfo;

    fn headers(&self) -> HashMap<String, String>;

    fn filter_list(&self) -> Vec<Input>;

    fn get_preferences(&self) -> Result<Vec<Input>>;

    fn set_preferences(&self, preferences: Vec<Input>) -> Result<()>;

    async fn get_popular_manga(&self, page: i64) -> Result<Vec<MangaInfo>>;

    async fn get_latest_manga(&self, page: i64) -> Result<Vec<MangaInfo>>;

    async fn search_manga(
        &self,
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    ) -> Result<Vec<MangaInfo>>;

    async fn get_manga_detail(&self, path: String) -> Result<MangaInfo>;

    async fn get_chapters(&self, path: String) -> Result<Vec<ChapterInfo>>;

    async fn get_pages(&self, path: String) -> Result<Vec<String>>;
}
