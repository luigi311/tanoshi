use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDateTime;

use thiserror::Error;

use crate::domain::entities::history::{
    HistoryBounds, HistoryChapter, HistoryCursor, HistoryPageInfo,
};

#[derive(Debug, Error)]
pub enum HistoryRepositoryError {
    #[error("database error: {0}")]
    DbError(#[from] sqlx::Error),
}

#[async_trait]
pub trait HistoryRepository: Send + Sync {
    async fn get_first_history_chapters(
        &self,
        user_id: i64,
        bounds: HistoryBounds,
        first: i32,
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError>;

    async fn get_last_history_chapters(
        &self,
        user_id: i64,
        bounds: HistoryBounds,
        last: i32,
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError>;

    async fn get_history_chapters(
        &self,
        user_id: i64,
        bounds: HistoryBounds,
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError>;

    async fn get_history_page_info(
        &self,
        user_id: i64,
        first: HistoryCursor,
        last: HistoryCursor,
    ) -> Result<HistoryPageInfo, HistoryRepositoryError>;

    async fn get_last_read_at_by_manga_ids(
        &self,
        user_id: i64,
        manga_ids: &[i64],
    ) -> Result<HashMap<i64, NaiveDateTime>, HistoryRepositoryError>;

    async fn get_history_chapters_by_chapter_ids(
        &self,
        user_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError>;

    async fn insert_history_chapter(
        &self,
        user_id: i64,
        chapter_id: i64,
        page: i64,
        is_complete: bool,
    ) -> Result<(), HistoryRepositoryError>;

    async fn insert_history_chapters_as_completed(
        &self,
        user_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), HistoryRepositoryError>;

    async fn delete_chapters_from_history(
        &self,
        user_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), HistoryRepositoryError>;

    async fn get_unread_chapters_by_manga_ids(
        &self,
        user_id: i64,
        manga_ids: &[i64],
    ) -> Result<HashMap<i64, i64>, HistoryRepositoryError>;

    async fn get_next_chapter_by_manga_id(
        &self,
        user_id: i64,
        manga_id: i64,
    ) -> Result<Option<i64>, HistoryRepositoryError>;
}
