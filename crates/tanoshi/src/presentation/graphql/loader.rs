use super::{common::ReadProgress, manga::Manga};
use crate::domain::{
    entities::download::DownloadQueueEntry,
    repositories::{
        download::DownloadRepository, history::HistoryRepository, library::LibraryRepository,
        manga::MangaRepository, tracker::TrackerRepository,
    },
};
use async_graphql::{Result, dataloader::Loader};
use chrono::NaiveDateTime;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

fn group_keys_by_user<K>(keys: &[K], user_id: impl Fn(&K) -> i64) -> HashMap<i64, Vec<&K>> {
    let mut groups = HashMap::<i64, Vec<&K>>::new();
    for key in keys {
        groups.entry(user_id(key)).or_default().push(key);
    }
    groups
}

pub struct DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    history_repo: H,
    library_repo: L,
    manga_repo: M,
    tracker_repo: T,
    download_repo: D,
}

impl<H, L, M, T, D> DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    pub fn new(
        history_repo: H,
        library_repo: L,
        manga_repo: M,
        tracker_repo: T,
        download_repo: D,
    ) -> Self {
        Self {
            history_repo,
            library_repo,
            manga_repo,
            tracker_repo,
            download_repo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserFavoriteId(pub i64, pub i64);

impl<H, L, M, T, D> Loader<UserFavoriteId> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = bool;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserFavoriteId],
    ) -> Result<HashMap<UserFavoriteId, Self::Value>, Self::Error> {
        let mut res = HashMap::new();
        // The schema shares this loader, so one batch can contain multiple users.
        for (user_id, keys) in group_keys_by_user(keys, |key| key.0) {
            let manga_id_set: HashSet<i64> = keys.iter().map(|key| key.1).collect();
            let manga = self
                .library_repo
                .get_manga_from_library(user_id)
                .await
                .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?;
            res.extend(manga.into_iter().map(|manga| {
                (
                    UserFavoriteId(user_id, manga.id),
                    manga_id_set.contains(&manga.id),
                )
            }));
        }

        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserFavoritePath(pub i64, pub String);

impl<H, L, M, T, D> Loader<UserFavoritePath> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = bool;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserFavoritePath],
    ) -> Result<HashMap<UserFavoritePath, Self::Value>, Self::Error> {
        let mut res = HashMap::new();
        for (user_id, keys) in group_keys_by_user(keys, |key| key.0) {
            let manga_path_set: HashSet<String> = keys.iter().map(|key| key.1.clone()).collect();
            let manga = self
                .library_repo
                .get_manga_from_library(user_id)
                .await
                .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?;
            res.extend(manga.into_iter().map(|manga| {
                let is_library = manga_path_set.contains(&manga.path);
                (UserFavoritePath(user_id, manga.path), is_library)
            }));
        }

        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserLastReadId(pub i64, pub i64);

impl<H, L, M, T, D> Loader<UserLastReadId> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = NaiveDateTime;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserLastReadId],
    ) -> Result<HashMap<UserLastReadId, Self::Value>, Self::Error> {
        let mut res = HashMap::new();
        for (user_id, keys) in group_keys_by_user(keys, |key| key.0) {
            let manga_ids: Vec<i64> = keys.iter().map(|key| key.1).collect();
            let last_read_at = self
                .history_repo
                .get_last_read_at_by_manga_ids(user_id, &manga_ids)
                .await
                .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?;
            res.extend(
                last_read_at
                    .into_iter()
                    .map(|(manga_id, read_at)| (UserLastReadId(user_id, manga_id), read_at)),
            );
        }

        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserUnreadChaptersId(pub i64, pub i64);

impl<H, L, M, T, D> Loader<UserUnreadChaptersId> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = i64;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserUnreadChaptersId],
    ) -> Result<HashMap<UserUnreadChaptersId, Self::Value>, Self::Error> {
        let mut res = HashMap::new();
        for (user_id, keys) in group_keys_by_user(keys, |key| key.0) {
            let manga_ids: Vec<i64> = keys.iter().map(|key| key.1).collect();
            let unread = self
                .history_repo
                .get_unread_chapters_by_manga_ids(user_id, &manga_ids)
                .await
                .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?;
            res.extend(
                unread
                    .into_iter()
                    .map(|(manga_id, count)| (UserUnreadChaptersId(user_id, manga_id), count)),
            );
        }
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserHistoryId(pub i64, pub i64);

impl<H, L, M, T, D> Loader<UserHistoryId> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = ReadProgress;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserHistoryId],
    ) -> Result<HashMap<UserHistoryId, Self::Value>, Self::Error> {
        let mut res = HashMap::new();
        for (user_id, keys) in group_keys_by_user(keys, |key| key.0) {
            let chapter_ids: Vec<i64> = keys.iter().map(|key| key.1).collect();
            let chapters = self
                .history_repo
                .get_history_chapters_by_chapter_ids(user_id, &chapter_ids)
                .await
                .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?;
            res.extend(chapters.into_iter().map(|chapter| {
                (
                    UserHistoryId(user_id, chapter.chapter_id),
                    ReadProgress {
                        at: chapter.read_at,
                        last_page: chapter.last_page_read,
                        is_complete: chapter.is_complete,
                    },
                )
            }));
        }
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MangaId(pub i64);

impl<H, L, M, T, D> Loader<MangaId> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = Manga;

    type Error = Arc<anyhow::Error>;

    async fn load(&self, keys: &[MangaId]) -> Result<HashMap<MangaId, Self::Value>, Self::Error> {
        let keys: Vec<i64> = keys.iter().map(|key| key.0).collect();
        let res = self
            .manga_repo
            .get_manga_by_ids(&keys)
            .await
            .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?
            .into_par_iter()
            .map(|m| (MangaId(m.id), m.into()))
            .collect();
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserTrackerMangaId(pub i64, pub i64);

impl<H, L, M, T, D> Loader<UserTrackerMangaId> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = Vec<(String, Option<String>)>;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserTrackerMangaId],
    ) -> Result<HashMap<UserTrackerMangaId, Self::Value>, Self::Error> {
        let mut res = HashMap::<_, Self::Value>::new();
        for (user_id, keys) in group_keys_by_user(keys, |key| key.0) {
            let manga_ids: Vec<i64> = keys.iter().map(|key| key.1).collect();
            let manga = self
                .tracker_repo
                .get_tracked_manga_id_by_manga_ids(user_id, &manga_ids)
                .await
                .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?;
            for manga in manga {
                res.entry(UserTrackerMangaId(user_id, manga.manga_id))
                    .or_default()
                    .push((manga.tracker, manga.tracker_manga_id));
            }
        }

        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserCategoryId(pub i64, pub Option<i64>);

impl<H, L, M, T, D> Loader<UserCategoryId> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = i64;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[UserCategoryId],
    ) -> Result<HashMap<UserCategoryId, Self::Value>, Self::Error> {
        let mut res = HashMap::new();
        for (user_id, _) in group_keys_by_user(keys, |key| key.0) {
            let categories = self
                .library_repo
                .get_category_count(user_id)
                .await
                .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?;
            res.extend(
                categories
                    .into_iter()
                    .map(|(category_id, count)| (UserCategoryId(user_id, category_id), count)),
            );
        }
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChapterDownloadQueueId(pub i64);

impl<H, L, M, T, D> Loader<ChapterDownloadQueueId> for DatabaseLoader<H, L, M, T, D>
where
    H: HistoryRepository + 'static,
    L: LibraryRepository + 'static,
    M: MangaRepository + 'static,
    T: TrackerRepository + 'static,
    D: DownloadRepository + 'static,
{
    type Value = DownloadQueueEntry;

    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[ChapterDownloadQueueId],
    ) -> Result<HashMap<ChapterDownloadQueueId, Self::Value>, Self::Error> {
        let chapter_ids: Vec<i64> = keys.iter().map(|key| key.0).collect();
        let res = self
            .download_repo
            .get_download_queue(&chapter_ids)
            .await
            .map_err(|e| Arc::new(anyhow::anyhow!("{e}")))?
            .into_par_iter()
            .map(|queue| (ChapterDownloadQueueId(queue.chapter_id), queue))
            .collect();
        Ok(res)
    }
}
