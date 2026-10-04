use std::{collections::HashMap, fs, path::PathBuf};

use async_graphql::dataloader::Loader;
use chrono::NaiveDateTime;
use tanoshi::{
    domain::repositories::history::HistoryRepository,
    infrastructure::{
        database::{Pool, establish_connection},
        domain::repositories::{
            download::DownloadRepositoryImpl, history::HistoryRepositoryImpl,
            library::LibraryRepositoryImpl, manga::MangaRepositoryImpl,
            tracker::TrackerRepositoryImpl,
        },
    },
    presentation::graphql::loader::{DatabaseLoader, UserLastReadId},
};

struct Fixture {
    dir: PathBuf,
    pool: Pool,
    repo: HistoryRepositoryImpl,
}

impl Fixture {
    async fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-last-read-at-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = establish_connection(dir.join("test.db").to_str().unwrap(), true)
            .await
            .unwrap();
        sqlx::raw_sql(
            r#"
            INSERT INTO user (id, username, password) VALUES
                (1, 'reader', 'unused'), (2, 'other-reader', 'unused');
            INSERT INTO manga (id, source_id, title, path, cover_url, date_added) VALUES
                (1, 1, 'Manga 1', '/manga/1', '', CURRENT_TIMESTAMP),
                (2, 1, 'Manga 2', '/manga/2', '', CURRENT_TIMESTAMP),
                (3, 1, 'Only read by user 2', '/manga/3', '', CURRENT_TIMESTAMP),
                (4, 1, 'Unread manga', '/manga/4', '', CURRENT_TIMESTAMP),
                (5, 1, 'Unrequested manga', '/manga/5', '', CURRENT_TIMESTAMP);
            INSERT INTO chapter (id, source_id, manga_id, title, path, number, uploaded, date_added) VALUES
                (1, 1, 1, 'Chapter 1', '/chapter/1', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (2, 1, 1, 'Chapter 2', '/chapter/2', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (3, 1, 1, 'Chapter 3', '/chapter/3', 3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (4, 1, 2, 'Chapter 1', '/chapter/4', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (5, 1, 3, 'Chapter 1', '/chapter/5', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (6, 1, 4, 'Chapter 1', '/chapter/6', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                (7, 1, 5, 'Chapter 1', '/chapter/7', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);
            INSERT INTO user_history (user_id, chapter_id, read_at) VALUES
                (1, 1, '2026-09-10 12:00:00.123456789'),
                (1, 2, '2026-09-01 12:00:00'),
                (1, 3, '2026-09-10 12:00:00.123456788'),
                (1, 4, '2026-09-05 12:00:00'),
                (1, 7, '2026-10-02 12:00:00'),
                (2, 2, '2026-10-01 12:00:00'),
                (2, 5, '2026-09-30 12:00:00');
            "#,
        )
        .execute(pool.write())
        .await
        .unwrap();

        let repo = HistoryRepositoryImpl::new(pool.clone());
        Self { dir, pool, repo }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn timestamp(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f").unwrap()
}

#[tokio::test]
async fn last_read_at_selects_latest_timestamp_and_scopes_history() {
    let fixture = Fixture::new().await;
    // Rereading an earlier chapter must win over a higher chapter ID/number.
    // Duplicate, unread, and nonexistent manga IDs must not add result rows.
    let ids = [1, 1, 2, 3, 4, 999];
    let result = fixture
        .repo
        .get_last_read_at_by_manga_ids(1, &ids)
        .await
        .unwrap();
    assert_eq!(
        result,
        HashMap::from([
            (1, timestamp("2026-09-10 12:00:00.123456789")),
            (2, timestamp("2026-09-05 12:00:00")),
        ])
    );

    let other_user = fixture
        .repo
        .get_last_read_at_by_manga_ids(2, &ids)
        .await
        .unwrap();
    assert_eq!(
        other_user,
        HashMap::from([
            (1, timestamp("2026-10-01 12:00:00")),
            (3, timestamp("2026-09-30 12:00:00")),
        ])
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn last_read_at_loader_returns_latest_timestamp_for_each_manga() {
    let fixture = Fixture::new().await;
    let loader = DatabaseLoader::new(
        fixture.repo.clone(),
        LibraryRepositoryImpl::new(fixture.pool.clone()),
        MangaRepositoryImpl::new(fixture.pool.clone()),
        TrackerRepositoryImpl::new(fixture.pool.clone(), None, None),
        DownloadRepositoryImpl::new(fixture.pool.clone()),
    );
    let result = loader
        .load(&[
            UserLastReadId(1, 1),
            UserLastReadId(1, 2),
            UserLastReadId(1, 3),
            UserLastReadId(1, 4),
            UserLastReadId(1, 999),
        ])
        .await
        .unwrap();
    assert_eq!(
        result,
        HashMap::from([
            (
                UserLastReadId(1, 1),
                timestamp("2026-09-10 12:00:00.123456789"),
            ),
            (UserLastReadId(1, 2), timestamp("2026-09-05 12:00:00")),
        ])
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn last_read_at_lookup_returns_no_values_for_empty_or_unread_ids() {
    let fixture = Fixture::new().await;
    for ids in [&[][..], &[3, 4, 999][..]] {
        assert!(
            fixture
                .repo
                .get_last_read_at_by_manga_ids(1, ids)
                .await
                .unwrap()
                .is_empty()
        );
    }
    fixture.pool.close().await;
}
