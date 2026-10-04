use std::{collections::HashMap, fs, path::PathBuf, time::Duration};

use async_graphql::dataloader::DataLoader;
use chrono::NaiveDateTime;
use tanoshi::{
    domain::repositories::tracker::TrackerRepository,
    infrastructure::{
        database::{Pool, establish_connection},
        domain::repositories::{
            download::DownloadRepositoryImpl, history::HistoryRepositoryImpl,
            library::LibraryRepositoryImpl, manga::MangaRepositoryImpl,
            tracker::TrackerRepositoryImpl,
        },
    },
    presentation::graphql::{
        loader::{
            UserCategoryId, UserFavoriteId, UserFavoritePath, UserHistoryId, UserLastReadId,
            UserTrackerMangaId, UserUnreadChaptersId,
        },
        schema::DatabaseLoader,
    },
};

struct Fixture {
    dir: PathBuf,
    pool: Pool,
}

impl Fixture {
    async fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-user-scoped-loaders-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = establish_connection(dir.join("test.db").to_str().unwrap(), true)
            .await
            .unwrap();
        sqlx::raw_sql(
            r#"
            INSERT INTO user (id, username, password) VALUES
                (1, 'reader-1', 'unused'), (2, 'reader-2', 'unused'),
                (3, 'empty-reader', 'unused');
            INSERT INTO manga (id, source_id, title, author, genre, path, cover_url, date_added) VALUES
                (1, 1, 'Shared manga', '[]', '[]', '/manga/1', '', CURRENT_TIMESTAMP),
                (2, 1, 'User 1 favorite', '[]', '[]', '/manga/2', '', CURRENT_TIMESTAMP),
                (3, 1, 'User 2 favorite', '[]', '[]', '/manga/3', '', CURRENT_TIMESTAMP),
                (4, 1, 'Unread manga', '[]', '[]', '/manga/4', '', CURRENT_TIMESTAMP);
            INSERT INTO chapter (id, source_id, manga_id, title, path, number, uploaded, date_added) VALUES
                (1, 1, 1, 'Chapter 1', '/chapter/1', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (2, 1, 1, 'Chapter 2', '/chapter/2', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (3, 1, 2, 'Chapter 1', '/chapter/3', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (4, 1, 3, 'Chapter 1', '/chapter/4', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (5, 1, 4, 'Chapter 1', '/chapter/5', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);
            INSERT INTO user_history (user_id, chapter_id, last_page, read_at, is_complete) VALUES
                (1, 1, 2, '2026-09-10 12:00:00', false),
                (1, 2, 7, '2026-09-01 12:00:00', false),
                (1, 3, 3, '2026-09-11 12:00:00', true),
                (1, 4, 4, '2026-09-12 12:00:00', false),
                (2, 1, 10, '2026-10-01 12:00:00', true),
                (2, 2, 8, '2026-09-30 12:00:00', false),
                (2, 3, 13, '2026-10-02 12:00:00', false),
                (2, 4, 14, '2026-10-03 12:00:00', true);
            INSERT INTO user_library (id, user_id, manga_id) VALUES
                (11, 1, 1), (12, 1, 2),
                (21, 2, 1), (22, 2, 3), (23, 2, 4);
            INSERT INTO user_category (id, user_id, name) VALUES
                (1, 1, 'User 1 shelf'), (2, 2, 'User 2 shelf');
            INSERT INTO library_category (library_id, category_id) VALUES (11, 1), (21, 2);
            INSERT INTO tracker_credential (user_id, tracker, access_token) VALUES
                (1, 'anilist', 'unused'), (1, 'mal', 'unused'),
                (2, 'anilist', 'unused'), (2, 'mal', 'unused');
            INSERT INTO tracker_manga (user_id, manga_id, tracker, tracker_manga_id) VALUES
                (1, 1, 'anilist', 'user-1-anilist-1'),
                (1, 1, 'mal', 'user-1-mal-1'),
                (1, 2, 'mal', 'user-1-mal-2'),
                (2, 1, 'anilist', 'user-2-anilist-1'),
                (2, 1, 'mal', 'user-2-mal-1'),
                (2, 2, 'anilist', 'user-2-anilist-2'),
                (2, 2, 'mal', 'user-2-mal-2'),
                (2, 3, 'anilist', 'user-2-anilist-3'),
                (2, 3, 'mal', 'user-2-mal-3');
            "#,
        )
        .execute(pool.write())
        .await
        .unwrap();
        Self { dir, pool }
    }

    fn loader(&self) -> DataLoader<DatabaseLoader> {
        // Joined requests share a loader, as requests do on the production schema.
        // Widen its delay so both users reliably enter the same batch.
        DataLoader::new(
            DatabaseLoader::new(
                HistoryRepositoryImpl::new(self.pool.clone()),
                LibraryRepositoryImpl::new(self.pool.clone()),
                MangaRepositoryImpl::new(self.pool.clone()),
                TrackerRepositoryImpl::new(self.pool.clone(), None, None),
                DownloadRepositoryImpl::new(self.pool.clone()),
            ),
            tokio::spawn,
        )
        .delay(Duration::from_millis(10))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn timestamp(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").unwrap()
}

#[tokio::test]
async fn concurrent_last_read_requests_keep_users_separate() {
    let fixture = Fixture::new().await;
    let loader = fixture.loader();
    let (shared_1, shared_2, different_1, different_2, unread, empty_user) = tokio::join!(
        loader.load_one(UserLastReadId(1, 1)),
        loader.load_one(UserLastReadId(2, 1)),
        loader.load_one(UserLastReadId(1, 2)),
        loader.load_one(UserLastReadId(2, 3)),
        loader.load_one(UserLastReadId(1, 4)),
        loader.load_one(UserLastReadId(3, 1)),
    );
    assert_eq!(shared_1.unwrap(), Some(timestamp("2026-09-10 12:00:00")));
    assert_eq!(shared_2.unwrap(), Some(timestamp("2026-10-01 12:00:00")));
    assert_eq!(different_1.unwrap(), Some(timestamp("2026-09-11 12:00:00")));
    assert_eq!(different_2.unwrap(), Some(timestamp("2026-10-03 12:00:00")));
    assert_eq!(unread.unwrap(), None);
    assert_eq!(empty_user.unwrap(), None);
    fixture.pool.close().await;
}

#[tokio::test]
async fn concurrent_favorite_id_requests_keep_users_separate() {
    let fixture = Fixture::new().await;
    let loader = fixture.loader();
    let (user_1, user_2) = tokio::join!(
        loader.load_many([
            UserFavoriteId(1, 1),
            UserFavoriteId(1, 2),
            UserFavoriteId(1, 3)
        ]),
        loader.load_many([
            UserFavoriteId(2, 1),
            UserFavoriteId(2, 2),
            UserFavoriteId(2, 3)
        ]),
    );
    assert_eq!(
        user_1.unwrap(),
        HashMap::from([(UserFavoriteId(1, 1), true), (UserFavoriteId(1, 2), true)])
    );
    assert_eq!(
        user_2.unwrap(),
        HashMap::from([(UserFavoriteId(2, 1), true), (UserFavoriteId(2, 3), true)])
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn concurrent_favorite_path_requests_keep_users_separate() {
    let fixture = Fixture::new().await;
    let loader = fixture.loader();
    let (user_1, user_2) = tokio::join!(
        loader.load_many([
            UserFavoritePath(1, 1, "/manga/1".into()),
            UserFavoritePath(1, 1, "/manga/2".into()),
            UserFavoritePath(1, 1, "/manga/3".into()),
        ]),
        loader.load_many([
            UserFavoritePath(2, 1, "/manga/1".into()),
            UserFavoritePath(2, 1, "/manga/2".into()),
            UserFavoritePath(2, 1, "/manga/3".into()),
        ]),
    );
    assert_eq!(
        user_1.unwrap(),
        HashMap::from([
            (UserFavoritePath(1, 1, "/manga/1".into()), true),
            (UserFavoritePath(1, 1, "/manga/2".into()), true),
        ])
    );
    assert_eq!(
        user_2.unwrap(),
        HashMap::from([
            (UserFavoritePath(2, 1, "/manga/1".into()), true),
            (UserFavoritePath(2, 1, "/manga/3".into()), true),
        ])
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn concurrent_unread_count_requests_keep_users_separate() {
    let fixture = Fixture::new().await;
    let loader = fixture.loader();
    let (user_1, user_2) = tokio::join!(
        loader.load_many([
            UserUnreadChaptersId(1, 1),
            UserUnreadChaptersId(1, 2),
            UserUnreadChaptersId(1, 3),
        ]),
        loader.load_many([
            UserUnreadChaptersId(2, 1),
            UserUnreadChaptersId(2, 2),
            UserUnreadChaptersId(2, 3),
        ]),
    );
    assert_eq!(
        user_1.unwrap(),
        HashMap::from([
            (UserUnreadChaptersId(1, 1), 2),
            (UserUnreadChaptersId(1, 3), 1)
        ])
    );
    assert_eq!(
        user_2.unwrap(),
        HashMap::from([
            (UserUnreadChaptersId(2, 1), 1),
            (UserUnreadChaptersId(2, 2), 1)
        ])
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn concurrent_read_progress_requests_keep_users_separate() {
    let fixture = Fixture::new().await;
    let loader = fixture.loader();
    let (user_1, user_2) = tokio::join!(
        loader.load_many([
            UserHistoryId(1, 1),
            UserHistoryId(1, 3),
            UserHistoryId(1, 4),
            UserHistoryId(1, 5),
        ]),
        loader.load_many([
            UserHistoryId(2, 1),
            UserHistoryId(2, 3),
            UserHistoryId(2, 4),
            UserHistoryId(2, 5),
        ]),
    );
    let mut progress = HashMap::new();
    for result in [user_1, user_2] {
        progress.extend(
            result
                .unwrap()
                .into_iter()
                .map(|(key, value)| (key, (value.at, value.last_page, value.is_complete))),
        );
    }
    assert_eq!(
        progress,
        HashMap::from([
            (
                UserHistoryId(1, 1),
                (timestamp("2026-09-10 12:00:00"), 2, false)
            ),
            (
                UserHistoryId(1, 3),
                (timestamp("2026-09-11 12:00:00"), 3, true)
            ),
            (
                UserHistoryId(1, 4),
                (timestamp("2026-09-12 12:00:00"), 4, false)
            ),
            (
                UserHistoryId(2, 1),
                (timestamp("2026-10-01 12:00:00"), 10, true)
            ),
            (
                UserHistoryId(2, 3),
                (timestamp("2026-10-02 12:00:00"), 13, false)
            ),
            (
                UserHistoryId(2, 4),
                (timestamp("2026-10-03 12:00:00"), 14, true)
            ),
        ])
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn concurrent_category_count_requests_keep_users_separate() {
    let fixture = Fixture::new().await;
    let loader = fixture.loader();
    let (user_1, user_2) = tokio::join!(
        loader.load_many([
            UserCategoryId(1, None),
            UserCategoryId(1, Some(1)),
            UserCategoryId(1, Some(2)),
        ]),
        loader.load_many([
            UserCategoryId(2, None),
            UserCategoryId(2, Some(1)),
            UserCategoryId(2, Some(2)),
        ]),
    );
    assert_eq!(
        user_1.unwrap(),
        HashMap::from([
            (UserCategoryId(1, None), 1),
            (UserCategoryId(1, Some(1)), 1)
        ])
    );
    assert_eq!(
        user_2.unwrap(),
        HashMap::from([
            (UserCategoryId(2, None), 2),
            (UserCategoryId(2, Some(2)), 1)
        ])
    );
    fixture.pool.close().await;
}

fn tracker_values(anilist: Option<&str>, mal: Option<&str>) -> Vec<(String, Option<String>)> {
    vec![
        ("anilist".into(), anilist.map(str::to_owned)),
        ("mal".into(), mal.map(str::to_owned)),
    ]
}

#[tokio::test]
async fn concurrent_tracker_requests_keep_users_and_all_trackers_separate() {
    let fixture = Fixture::new().await;
    let loader = fixture.loader();
    let (user_1, user_2) = tokio::join!(
        loader.load_many([
            UserTrackerMangaId(1, 1),
            UserTrackerMangaId(1, 2),
            UserTrackerMangaId(1, 3),
        ]),
        loader.load_many([
            UserTrackerMangaId(2, 1),
            UserTrackerMangaId(2, 2),
            UserTrackerMangaId(2, 3),
        ]),
    );
    let mut trackers = HashMap::new();
    for result in [user_1, user_2] {
        trackers.extend(result.unwrap().into_iter().map(|(key, mut values)| {
            values.sort();
            (key, values)
        }));
    }
    assert_eq!(
        trackers,
        HashMap::from([
            (
                UserTrackerMangaId(1, 1),
                tracker_values(Some("user-1-anilist-1"), Some("user-1-mal-1"))
            ),
            (
                UserTrackerMangaId(1, 2),
                tracker_values(None, Some("user-1-mal-2"))
            ),
            (UserTrackerMangaId(1, 3), tracker_values(None, None)),
            (
                UserTrackerMangaId(2, 1),
                tracker_values(Some("user-2-anilist-1"), Some("user-2-mal-1"))
            ),
            (
                UserTrackerMangaId(2, 2),
                tracker_values(Some("user-2-anilist-2"), Some("user-2-mal-2"))
            ),
            (
                UserTrackerMangaId(2, 3),
                tracker_values(Some("user-2-anilist-3"), Some("user-2-mal-3"))
            ),
        ])
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn single_manga_tracker_lookup_excludes_other_users_mappings() {
    let fixture = Fixture::new().await;
    let repo = TrackerRepositoryImpl::new(fixture.pool.clone(), None, None);
    for (user_id, expected) in [
        (1, tracker_values(None, Some("user-1-mal-2"))),
        (
            2,
            tracker_values(Some("user-2-anilist-2"), Some("user-2-mal-2")),
        ),
    ] {
        let mut values: Vec<_> = repo
            .get_tracked_manga_id(user_id, 2)
            .await
            .unwrap()
            .into_iter()
            .map(|manga| (manga.tracker, manga.tracker_manga_id))
            .collect();
        values.sort();
        assert_eq!(values, expected);
    }
    fixture.pool.close().await;
}
