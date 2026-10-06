use std::{borrow::Cow, fs, path::PathBuf};

use async_graphql::{EmptyMutation, EmptySubscription, Request, Schema, connection::CursorType};
use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tanoshi::{
    domain::{
        entities::history::{HistoryBounds, HistoryCursor},
        repositories::{
            chapter::ChapterRepository, history::HistoryRepository, library::LibraryRepository,
        },
        services::history::HistoryService,
    },
    infrastructure::{
        auth::Claims,
        database::{Pool, establish_connection},
        domain::repositories::{
            chapter::ChapterRepositoryImpl, history::HistoryRepositoryImpl,
            library::LibraryRepositoryImpl,
        },
    },
    presentation::graphql::library::LibraryRoot,
};

type HistorySchema = Schema<LibraryRoot, EmptyMutation, EmptySubscription>;
type Summary = (i64, i64, i64, NaiveDateTime);

const SEED: &str = r#"
INSERT INTO user (id, username, password) VALUES (1, 'reader', 'unused'), (2, 'other', 'unused');
WITH RECURSIVE n(id) AS (VALUES(1) UNION ALL SELECT id + 1 FROM n WHERE id < 6)
INSERT INTO manga (id, source_id, title, path, cover_url, date_added)
SELECT id, 1, 'Manga ' || id, '/manga/' || id, '', CURRENT_TIMESTAMP FROM n;
INSERT INTO chapter (id, source_id, manga_id, title, path, number, uploaded, date_added) VALUES
    (1, 1, 1, 'First', '/chapter/1', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (2, 1, 1, 'Second', '/chapter/2', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (3, 1, 2, 'Third', '/chapter/3', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (4, 1, 3, 'Fourth', '/chapter/4', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (5, 1, 4, 'Fifth', '/chapter/5', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (6, 1, 5, 'Sixth', '/chapter/6', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (7, 1, 6, 'Destination first', '/chapter/7', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (8, 1, 1, 'Second duplicate', '/chapter/8', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (9, 1, 6, 'Destination second', '/chapter/9', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);
INSERT INTO user_library(user_id, manga_id) VALUES (1, 1), (2, 1);
INSERT INTO user_history(user_id, chapter_id, last_page, read_at, is_complete) VALUES
    (1, 1, 1, '2026-09-08 12:00:00', false),
    (1, 2, 2, '2026-09-10 12:00:00.123456789', false),
    (1, 3, 3, '2026-09-10 12:00:00.123456789', false),
    (1, 4, 4, '2026-09-10 12:00:00.123456790', false),
    (1, 5, 5, '2026-09-10 12:00:00.123456', false),
    (1, 6, 6, '2026-09-10 12:00:00', false),
    (1, 8, 8, '2026-09-10 12:00:00.123456789', true),
    (2, 1, 9, '2026-09-20 12:00:00', true);
"#;

const LEGACY_TIMESTAMPS: &[(&str, &str)] = &[
    (
        "2022-01-05 10:00:00.123456789-05:00",
        "2022-01-05 15:00:00.123456789",
    ),
    (
        "2022-01-05 10:00:00.123456-05:00",
        "2022-01-05 15:00:00.123456000",
    ),
    (
        "2022-01-05 10:00:00.123+02:00",
        "2022-01-05 08:00:00.123000000",
    ),
    (
        "2022-01-05 10:00:00.123456+05:30",
        "2022-01-05 04:30:00.123456000",
    ),
    (
        "2022-01-05 10:00:00.123456-03:30",
        "2022-01-05 13:30:00.123456000",
    ),
    ("2022-01-05 10:00:00-05:00", "2022-01-05 15:00:00.000000000"),
    (
        "2022-01-05T10:00:00.123456789Z",
        "2022-01-05 10:00:00.123456789",
    ),
    (
        "2022-01-05T10:00:00.123456Z",
        "2022-01-05 10:00:00.123456000",
    ),
    ("2022-01-05T10:00:00.123Z", "2022-01-05 10:00:00.123000000"),
    ("2022-01-05T10:00:00Z", "2022-01-05 10:00:00.000000000"),
    ("2022-01-05 10:00:00+00:00", "2022-01-05 10:00:00.000000000"),
    (
        "2022-01-05 10:00:00.123456",
        "2022-01-05 10:00:00.123456000",
    ),
    (
        "2022-01-05 10:00:00.999999999",
        "2022-01-05 10:00:00.999999999",
    ),
    (
        "2022-01-05 10:00:00.999999999-05:00",
        "2022-01-05 15:00:00.999999999",
    ),
    (
        "2022-01-05 10:00:00.999999999+02:00",
        "2022-01-05 08:00:00.999999999",
    ),
    (
        "2022-01-05T23:59:59.999999999Z",
        "2022-01-05 23:59:59.999999999",
    ),
    (
        "2022-01-05 23:59:59.999999999-05:00",
        "2022-01-06 04:59:59.999999999",
    ),
    (
        "2022-01-01T00:00:00.123456+02:00",
        "2021-12-31 22:00:00.123456000",
    ),
    ("2022-01-05", "2022-01-05 00:00:00.000000000"),
];

async fn seed_legacy_history(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO user(id, username, password) VALUES(3, 'legacy', 'unused')")
        .execute(pool)
        .await
        .unwrap();
    for (index, (timestamp, _)) in LEGACY_TIMESTAMPS.iter().enumerate() {
        let id = 100 + index as i64;
        sqlx::query("INSERT INTO manga(id, source_id, title, path, cover_url, date_added) VALUES(?, 1, 'Legacy', ?, '', CURRENT_TIMESTAMP)")
            .bind(id).bind(format!("/manga/{id}")).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO chapter(id, source_id, manga_id, title, path, number, uploaded, date_added) VALUES(?, 1, ?, 'Legacy', ?, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(id).bind(id).bind(format!("/chapter/{id}")).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO user_history(user_id, chapter_id, last_page, read_at, is_complete) VALUES(3, ?, 7, ?, true)")
            .bind(id).bind(timestamp).execute(pool).await.unwrap();
    }
    // Its local wall time is later than chapter 100, but its UTC time is earlier.
    sqlx::raw_sql(r#"
        INSERT INTO chapter(id, source_id, manga_id, title, path, number, uploaded, date_added)
        VALUES(999, 1, 100, 'Earlier in UTC', '/chapter/999', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);
        INSERT INTO user_history(user_id, chapter_id, read_at)
        VALUES(3, 999, '2022-01-05 11:00:00.123456789+02:00');
    "#).execute(pool).await.unwrap();
}

async fn assert_legacy_history_normalized(f: &Fixture) {
    // Decode through sqlx as well as checking the stored representation.
    let actual: Vec<(i64, String, NaiveDateTime)> = sqlx::query_as(
        "SELECT chapter_id, read_at, read_at FROM user_history WHERE user_id=3 ORDER BY chapter_id",
    )
    .fetch_all(f.pool.read())
    .await
    .unwrap();
    assert_eq!(actual.len(), LEGACY_TIMESTAMPS.len() + 1);
    let mut expected_order = vec![];
    for (index, (_, expected)) in LEGACY_TIMESTAMPS.iter().enumerate() {
        let id = 100 + index as i64;
        let timestamp = NaiveDateTime::parse_from_str(expected, "%Y-%m-%d %H:%M:%S%.f").unwrap();
        assert_eq!(actual[index], (id, expected.to_string(), timestamp));
        expected_order.push((timestamp, id));
    }
    assert_eq!(actual.last().unwrap().1, "2022-01-05 09:00:00.123456789");
    expected_order.sort_by(|a, b| b.cmp(a));
    let page = f.page(3, "").await;
    assert_eq!(
        ids(&page),
        expected_order.iter().map(|(_, id)| *id).collect::<Vec<_>>()
    );
    for (edge, (timestamp, id)) in page["edges"].as_array().unwrap().iter().zip(expected_order) {
        assert_eq!(edge["node"]["chapterId"], id);
        assert_eq!(edge["node"]["lastPageRead"], 7);
        assert_eq!(
            HistoryCursor::decode_cursor(edge["cursor"].as_str().unwrap()).unwrap(),
            HistoryCursor {
                read_at: timestamp,
                manga_id: id
            }
        );
    }
    f.assert_consistent().await;
}

struct Fixture {
    dir: PathBuf,
    pool: Pool,
    history: HistoryRepositoryImpl,
    schema: HistorySchema,
}

impl Fixture {
    async fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-history-pagination-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = establish_connection(dir.join("test.db").to_str().unwrap(), true)
            .await
            .unwrap();
        sqlx::raw_sql(SEED).execute(pool.write()).await.unwrap();
        Self::from_pool(dir, pool)
    }

    fn from_pool(dir: PathBuf, pool: Pool) -> Self {
        let history = HistoryRepositoryImpl::new(pool.clone());
        let schema = Schema::build(LibraryRoot, EmptyMutation, EmptySubscription)
            .data(HistoryService::new(
                ChapterRepositoryImpl::new(pool.clone()),
                history.clone(),
            ))
            .finish();
        Self {
            dir,
            pool,
            history,
            schema,
        }
    }

    async fn page(&self, user_id: i64, arguments: &str) -> Value {
        let request = Request::new(format!(
            r#"{{ recentChapters{arguments} {{
            edges {{ cursor node {{ mangaId chapterId readAt lastPageRead }} }}
            pageInfo {{ hasPreviousPage hasNextPage startCursor endCursor }}
        }} }}"#
        ))
        .data(Claims {
            sub: user_id,
            username: "reader".into(),
            is_admin: false,
            exp: usize::MAX,
        });
        let response = self.schema.execute(request).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        response.data.into_json().unwrap()["recentChapters"].clone()
    }

    async fn summaries(&self) -> Vec<Summary> {
        sqlx::query_as("SELECT user_id, manga_id, chapter_id, read_at FROM user_manga_history ORDER BY user_id, manga_id")
            .fetch_all(self.pool.read()).await.unwrap()
    }

    async fn assert_consistent(&self) {
        let expected: Vec<Summary> = sqlx::query_as(
            r#"
            SELECT user_id, manga_id, chapter_id, read_at FROM (
                SELECT h.user_id, c.manga_id, h.chapter_id, h.read_at,
                    ROW_NUMBER() OVER (PARTITION BY h.user_id, c.manga_id
                        ORDER BY h.read_at DESC, h.chapter_id DESC) AS position
                FROM user_history h JOIN chapter c ON c.id = h.chapter_id
                JOIN manga m ON m.id = c.manga_id JOIN user u ON u.id = h.user_id
            ) WHERE position = 1 ORDER BY user_id, manga_id
        "#,
        )
        .fetch_all(self.pool.read())
        .await
        .unwrap();
        assert_eq!(self.summaries().await, expected);
    }

    async fn close(self) {
        self.pool.close().await;
        fs::remove_dir_all(&self.dir).unwrap();
    }
}

fn ids(page: &Value) -> Vec<i64> {
    page["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"]["mangaId"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn graphql_history_paginates_ties_and_nanoseconds_in_both_directions() {
    let f = Fixture::new().await;
    let first = f.page(1, "(first: 2)").await;
    assert_eq!(ids(&first), [3, 2]);
    assert_eq!(first["pageInfo"]["hasPreviousPage"], false);
    assert_eq!(first["pageInfo"]["hasNextPage"], true);
    let after = first["pageInfo"]["endCursor"].as_str().unwrap();
    let decoded = HistoryCursor::decode_cursor(after).unwrap();
    assert_eq!(
        decoded.read_at.and_utc().timestamp_subsec_nanos(),
        123456789
    );
    let second = f.page(1, &format!("(first: 2, after: {after:?})")).await;
    assert_eq!(ids(&second), [1, 4]);
    assert_eq!(second["edges"][0]["node"]["chapterId"], 8);
    assert_eq!(second["edges"][0]["node"]["lastPageRead"], 8);
    assert_eq!(second["pageInfo"]["hasPreviousPage"], true);
    assert_eq!(second["pageInfo"]["hasNextPage"], true);
    let after = second["pageInfo"]["endCursor"].as_str().unwrap();
    let tail = f.page(1, &format!("(first: 2, after: {after:?})")).await;
    assert_eq!(ids(&tail), [5]);
    assert_eq!(tail["pageInfo"]["hasPreviousPage"], true);
    assert_eq!(tail["pageInfo"]["hasNextPage"], false);

    let last = f.page(1, "(last: 2)").await;
    assert_eq!(ids(&last), [4, 5]);
    assert_eq!(last["pageInfo"]["hasNextPage"], false);
    let before = last["pageInfo"]["startCursor"].as_str().unwrap();
    let previous = f.page(1, &format!("(last: 2, before: {before:?})")).await;
    assert_eq!(ids(&previous), [2, 1]);
    let before = previous["pageInfo"]["startCursor"].as_str().unwrap();
    let head = f.page(1, &format!("(last: 2, before: {before:?})")).await;
    assert_eq!(ids(&head), [3]);
    assert_eq!(head["pageInfo"]["hasPreviousPage"], false);

    let bounds_after = first["pageInfo"]["endCursor"].as_str().unwrap();
    let bounds_before = tail["pageInfo"]["startCursor"].as_str().unwrap();
    assert_eq!(
        ids(&f
            .page(
                1,
                &format!("(after: {bounds_after:?}, before: {bounds_before:?})")
            )
            .await),
        [1, 4]
    );
    assert_eq!(ids(&f.page(1, "").await), [3, 2, 1, 4, 5]);
    let other = f.page(2, "(first: 2)").await;
    assert_eq!(ids(&other), [1]);
    assert_eq!(other["edges"][0]["node"]["chapterId"], 1);
    assert_eq!(other["pageInfo"]["hasPreviousPage"], false);
    assert_eq!(other["pageInfo"]["hasNextPage"], false);
    for page in [
        f.page(1, "(first: 0)").await,
        f.page(999, "(first: 2)").await,
    ] {
        assert!(ids(&page).is_empty());
        assert_eq!(page["pageInfo"]["hasPreviousPage"], false);
        assert_eq!(page["pageInfo"]["hasNextPage"], false);
    }
    f.assert_consistent().await;
    f.close().await;
}

#[tokio::test]
async fn reading_bulk_completion_and_unread_update_latest_history() {
    let f = Fixture::new().await;
    f.history
        .insert_history_chapter(1, 1, 7, true)
        .await
        .unwrap();
    f.history
        .insert_history_chapter(1, 1, 9, false)
        .await
        .unwrap();
    let rows = f
        .history
        .get_history_chapters(1, HistoryBounds::default())
        .await
        .unwrap();
    let row = rows.iter().find(|row| row.manga_id == 1).unwrap();
    assert_eq!(row.chapter_id, 1);
    assert_eq!(row.last_page_read, 9);
    assert!(row.is_complete);
    f.history
        .insert_history_chapters_as_completed(1, &[3, 4])
        .await
        .unwrap();
    f.assert_consistent().await;
    // Removing a non-latest read must preserve the selected chapter.
    f.history
        .delete_chapters_from_history(1, &[2])
        .await
        .unwrap();
    f.assert_consistent().await;
    // Removing the latest read falls back to the next newest chapter.
    f.history
        .delete_chapters_from_history(1, &[1])
        .await
        .unwrap();
    let rows = f
        .history
        .get_history_chapters(1, HistoryBounds::default())
        .await
        .unwrap();
    assert_eq!(
        rows.iter()
            .find(|row| row.manga_id == 1)
            .unwrap()
            .chapter_id,
        8
    );
    f.history
        .delete_chapters_from_history(1, &[8])
        .await
        .unwrap();
    assert!(!ids(&f.page(1, "").await).contains(&1));
    assert_eq!(ids(&f.page(2, "").await), [1]);
    f.assert_consistent().await;
    f.close().await;
}

#[tokio::test]
async fn unread_counts_and_resume_queries_work_with_denormalized_history() {
    let f = Fixture::new().await;
    // Resume has its own chapter ordering; give it an unambiguous latest read.
    sqlx::query("UPDATE user_history SET read_at = '2026-09-15 12:00:00' WHERE user_id = 1 AND chapter_id = 8")
        .execute(f.pool.write()).await.unwrap();
    let unread = f
        .history
        .get_unread_chapters_by_manga_ids(1, &[1, 2, 6])
        .await
        .unwrap();
    assert_eq!(unread[&1], 2);
    assert_eq!(unread[&2], 1);
    assert_eq!(unread[&6], 2);
    for (user_id, manga_id, expected) in [(1, 1, 1), (1, 2, 3), (1, 6, 7), (2, 1, 2)] {
        assert_eq!(
            f.history
                .get_next_chapter_by_manga_id(user_id, manga_id)
                .await
                .unwrap(),
            Some(expected)
        );
    }
    f.history
        .insert_history_chapters_as_completed(1, &[1, 2, 8])
        .await
        .unwrap();
    assert_eq!(
        f.history.get_next_chapter_by_manga_id(1, 1).await.unwrap(),
        None
    );
    assert!(
        !f.history
            .get_unread_chapters_by_manga_ids(1, &[1])
            .await
            .unwrap()
            .contains_key(&1)
    );
    f.assert_consistent().await;
    f.close().await;
}

#[tokio::test]
async fn direct_history_updates_and_chapter_refreshes_move_summaries() {
    let f = Fixture::new().await;
    // A timestamp moving backwards must select the next newest read.
    sqlx::query(
        "UPDATE user_history SET read_at = '2026-09-01' WHERE user_id = 1 AND chapter_id = 8",
    )
    .execute(f.pool.write())
    .await
    .unwrap();
    f.assert_consistent().await;
    assert_eq!(
        f.summaries()
            .await
            .iter()
            .find(|row| row.0 == 1 && row.1 == 1)
            .unwrap()
            .2,
        2
    );
    sqlx::query(
        "UPDATE user_history SET user_id = 2, chapter_id = 7 WHERE user_id = 1 AND chapter_id = 4",
    )
    .execute(f.pool.write())
    .await
    .unwrap();
    f.assert_consistent().await;
    let chapters = ChapterRepositoryImpl::new(f.pool.clone());
    let mut chapter = chapters.get_chapter_by_id(2).await.unwrap();
    chapter.manga_id = 6;
    chapters.insert_chapters(&[chapter]).await.unwrap();
    f.assert_consistent().await;
    assert_eq!(
        f.summaries()
            .await
            .iter()
            .find(|row| row.0 == 1 && row.1 == 6)
            .unwrap()
            .2,
        2
    );
    f.close().await;
}

#[tokio::test]
async fn latest_history_triggers_work_with_and_without_recursion() {
    for recursive in [false, true] {
        let f = Fixture::new().await;
        sqlx::query(&format!(
            "PRAGMA recursive_triggers = {}",
            i32::from(recursive)
        ))
        .execute(f.pool.write())
        .await
        .unwrap();
        for statement in [
            "INSERT INTO user_history(user_id, chapter_id, read_at) VALUES(1, 7, '2026-09-11 12:00:00.123')",
            "UPDATE user_history SET read_at = '2026-09-11 12:00:00.123456789' WHERE user_id = 1 AND chapter_id = 7",
            "UPDATE user_history SET user_id = 2, chapter_id = 9 WHERE user_id = 1 AND chapter_id = 7",
            "UPDATE chapter SET manga_id = 6 WHERE id = 8",
            "UPDATE user_history SET manga_id = 1 WHERE user_id = 1 AND chapter_id = 8",
            "DELETE FROM user_history WHERE user_id = 1 AND chapter_id = 8",
            "DELETE FROM manga WHERE id = 6",
            "DELETE FROM user WHERE id = 1",
        ] {
            sqlx::query(statement)
                .execute(f.pool.write())
                .await
                .unwrap();
            f.assert_consistent().await;
        }
        f.close().await;
    }
}

#[tokio::test]
async fn chapter_manga_and_user_deletions_keep_latest_history_consistent() {
    let f = Fixture::new().await;
    let chapters = ChapterRepositoryImpl::new(f.pool.clone());
    chapters.delete_chapter_by_id(8).await.unwrap();
    f.assert_consistent().await;
    assert_eq!(
        f.summaries()
            .await
            .iter()
            .find(|row| row.0 == 1 && row.1 == 1)
            .unwrap()
            .2,
        2
    );
    chapters.delete_chapter_by_ids(&[1, 2, 3]).await.unwrap();
    f.assert_consistent().await;
    assert!(f.page(2, "").await["edges"].as_array().unwrap().is_empty());
    sqlx::query("DELETE FROM manga WHERE id = 3")
        .execute(f.pool.write())
        .await
        .unwrap();
    f.assert_consistent().await;
    sqlx::query("DELETE FROM user WHERE id = 1")
        .execute(f.pool.write())
        .await
        .unwrap();
    f.assert_consistent().await;
    assert!(f.summaries().await.is_empty());
    f.close().await;
}

#[tokio::test]
async fn manga_migration_and_rollback_preserve_latest_reads_and_users() {
    let f = Fixture::new().await;
    let library = LibraryRepositoryImpl::new(f.pool.clone());
    let before = f.summaries().await;
    sqlx::raw_sql(
        r#"CREATE TRIGGER reject_history_migration BEFORE DELETE ON user_history
        WHEN OLD.user_id = 1 AND OLD.chapter_id = 1
        BEGIN SELECT RAISE(ABORT, 'test rollback'); END;"#,
    )
    .execute(f.pool.write())
    .await
    .unwrap();
    assert!(library.migrate_manga(1, 1, 6).await.is_err());
    assert_eq!(f.summaries().await, before);
    f.assert_consistent().await;
    sqlx::query("DROP TRIGGER reject_history_migration")
        .execute(f.pool.write())
        .await
        .unwrap();
    library.migrate_manga(1, 1, 6).await.unwrap();
    f.assert_consistent().await;
    let page = f.page(1, "").await;
    assert!(!ids(&page).contains(&1));
    let migrated = page["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| edge["node"]["mangaId"] == 6)
        .unwrap();
    assert_eq!(migrated["node"]["chapterId"], 9);
    assert_eq!(migrated["node"]["lastPageRead"], 8);
    assert_eq!(
        HistoryCursor::decode_cursor(migrated["cursor"].as_str().unwrap())
            .unwrap()
            .read_at
            .and_utc()
            .timestamp_subsec_nanos(),
        123456789
    );
    assert_eq!(ids(&f.page(2, "").await), [1]);
    f.close().await;
}

#[tokio::test]
async fn legacy_history_writes_normalize_offsets_without_rounding() {
    for recursive in [false, true] {
        let f = Fixture::new().await;
        sqlx::query(&format!(
            "PRAGMA recursive_triggers = {}",
            i32::from(recursive)
        ))
        .execute(f.pool.write())
        .await
        .unwrap();
        seed_legacy_history(f.pool.write()).await;
        assert_legacy_history_normalized(&f).await;
        for (index, (timestamp, _)) in LEGACY_TIMESTAMPS.iter().enumerate() {
            sqlx::query("UPDATE user_history SET read_at = ? WHERE user_id=3 AND chapter_id = ?")
                .bind(timestamp)
                .bind(100 + index as i64)
                .execute(f.pool.write())
                .await
                .unwrap();
        }
        assert_legacy_history_normalized(&f).await;
        f.close().await;
    }
}

#[tokio::test]
async fn existing_database_backfills_latest_reads_without_changing_precision() {
    let dir = std::env::temp_dir().join(format!(
        "tanoshi-history-upgrade-{:016x}",
        rand::random::<u64>()
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.db");
    let old = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.migrations = Cow::Owned(
        migrator
            .iter()
            .filter(|migration| migration.version < 202610040001)
            .cloned()
            .collect(),
    );
    migrator.run(&old).await.unwrap();
    sqlx::raw_sql(SEED).execute(&old).await.unwrap();
    seed_legacy_history(&old).await;
    old.close().await;
    let pool = establish_connection(path.to_str().unwrap(), false)
        .await
        .unwrap();
    let f = Fixture::from_pool(dir, pool);
    assert_legacy_history_normalized(&f).await;
    f.assert_consistent().await;
    let page = f.page(1, "").await;
    assert_eq!(ids(&page), [3, 2, 1, 4, 5]);
    assert_eq!(page["edges"][2]["node"]["chapterId"], 8);
    let cursor =
        HistoryCursor::decode_cursor(page["edges"][0]["cursor"].as_str().unwrap()).unwrap();
    assert_eq!(cursor.read_at.and_utc().timestamp_subsec_nanos(), 123456790);
    f.close().await;
}
