use std::{fs, path::PathBuf, time::Duration};

use futures::{future::BoxFuture, future::join_all, poll};

use super::{Pool, establish_connection};
use crate::{
    domain::{
        entities::{manga::Manga, tracker::Token},
        repositories::{
            chapter::ChapterRepository, download::DownloadRepository, history::HistoryRepository,
            library::LibraryRepository, manga::MangaRepository, tracker::TrackerRepository,
            user::UserRepository,
        },
    },
    infrastructure::domain::repositories::{
        chapter::ChapterRepositoryImpl, download::DownloadRepositoryImpl,
        history::HistoryRepositoryImpl, library::LibraryRepositoryImpl, manga::MangaRepositoryImpl,
        tracker::TrackerRepositoryImpl, user::UserRepositoryImpl,
    },
};

const TIMEOUT: Duration = Duration::from_secs(10);

struct Fixture {
    dir: PathBuf,
    pool: Pool,
}

impl Fixture {
    async fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-database-pools-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = establish_connection(dir.join("test.db").to_str().unwrap(), true)
            .await
            .unwrap();
        for sql in [
            "INSERT INTO user (id, username, password, is_admin) VALUES (1, 'admin', 'original', true)",
            "INSERT INTO manga (id, source_id, title, author, genre, path, cover_url, date_added) \
             VALUES (1, 1, 'Original manga', '[]', '[]', '/manga', '/cover', CURRENT_TIMESTAMP)",
            "INSERT INTO chapter (id, source_id, manga_id, title, path, number, uploaded, date_added) \
             VALUES (1, 1, 1, 'Chapter 1', '/chapter/1', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP), \
             (2, 1, 1, 'Chapter 2', '/chapter/2', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO download_queue \
             (source_id, source_name, manga_id, manga_title, chapter_id, chapter_title, rank, url, priority, date_added) \
             VALUES (1, 'Source', 1, 'Manga', 1, 'Chapter 1', 0, '/page', 0, unixepoch())",
        ] {
            sqlx::query(sql).execute(pool.write()).await.unwrap();
        }
        Self { dir, pool }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test]
async fn reader_connections_reject_data_changes() {
    let fixture = Fixture::new().await;
    assert_eq!(fixture.pool.read().options().get_max_connections(), 4);
    assert_eq!(fixture.pool.write().options().get_max_connections(), 1);
    let mut readers = Vec::new();
    for _ in 0..4 {
        readers.push(fixture.pool.read().acquire().await.unwrap());
    }
    // Check every connection, including readers created after startup.
    for reader in &mut readers {
        let query_only: i64 = sqlx::query_scalar("PRAGMA query_only")
            .fetch_one(&mut **reader)
            .await
            .unwrap();
        assert_eq!(query_only, 1);
        let error = sqlx::query("UPDATE user SET password = 'unexpected' WHERE id = 1")
            .execute(&mut **reader)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("8")
        );
    }
    drop(readers);
    assert_eq!(
        UserRepositoryImpl::new(fixture.pool.clone())
            .get_user_by_id(1)
            .await
            .unwrap()
            .password,
        "original"
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn repository_writers_wait_without_consuming_reader_capacity() {
    let fixture = Fixture::new().await;
    let mut tx = fixture
        .pool
        .write()
        .begin_with("BEGIN IMMEDIATE")
        .await
        .unwrap();
    sqlx::query("UPDATE user SET password = 'uncommitted' WHERE id = 1")
        .execute(&mut *tx)
        .await
        .unwrap();

    let user = UserRepositoryImpl::new(fixture.pool.clone());
    let manga = MangaRepositoryImpl::new(fixture.pool.clone());
    let chapter = ChapterRepositoryImpl::new(fixture.pool.clone());
    let library = LibraryRepositoryImpl::new(fixture.pool.clone());
    let history = HistoryRepositoryImpl::new(fixture.pool.clone());
    let tracker = TrackerRepositoryImpl::new(fixture.pool.clone(), None, None);
    let download = DownloadRepositoryImpl::new(fixture.pool.clone());
    let jobs: Vec<BoxFuture<'static, anyhow::Result<()>>> = vec![
        Box::pin(async move {
            user.update_password(1, "updated".into()).await?;
            Ok(())
        }),
        Box::pin(async move {
            manga
                .insert_manga(&mut Manga {
                    id: 1,
                    source_id: 1,
                    title: "Updated manga".into(),
                    path: "/manga".into(),
                    cover_url: "/cover".into(),
                    ..Manga::default()
                })
                .await?;
            Ok(())
        }),
        Box::pin(async move {
            chapter.delete_chapter_by_id(2).await?;
            Ok(())
        }),
        Box::pin(async move {
            // Mutations returning rows and a multi-statement transaction are writes too.
            let category = library.create_category(1, "Before rename").await?;
            let category_id = category.id.expect("created categories have an ID");
            library.rename_category(category_id, "After rename").await?;
            library
                .insert_manga_to_library(1, 1, &[category_id])
                .await?;
            Ok(())
        }),
        Box::pin(async move {
            history.insert_history_chapter(1, 1, 3, false).await?;
            Ok(())
        }),
        Box::pin(async move {
            tracker
                .insert_tracker_credential(
                    1,
                    "test",
                    Token {
                        token_type: "Bearer".into(),
                        access_token: "access".into(),
                        refresh_token: "refresh".into(),
                        expires_in: 3600,
                    },
                )
                .await?;
            Ok(())
        }),
        Box::pin(async move {
            download.delete_single_chapter_download_queue(1).await?;
            Ok(())
        }),
    ];
    let pending = join_all(jobs);
    tokio::pin!(pending);
    // Poll all seven repository operations while the sole writer is checked out.
    assert!(poll!(&mut pending).is_pending());
    let mut readers = Vec::new();
    for _ in 0..4 {
        let mut reader = tokio::time::timeout(TIMEOUT, fixture.pool.read().acquire())
            .await
            .expect("queued writers consumed reader capacity")
            .unwrap();
        let password: String = sqlx::query_scalar("SELECT password FROM user WHERE id = 1")
            .fetch_one(&mut *reader)
            .await
            .unwrap();
        assert_eq!(password, "original", "readers must see committed data");
        readers.push(reader);
    }
    assert_eq!(fixture.pool.read().size(), 4);
    assert_eq!(fixture.pool.write().size(), 1);
    assert_eq!(fixture.pool.write().num_idle(), 0);
    assert!(poll!(&mut pending).is_pending());
    drop(readers);
    tx.commit().await.unwrap();
    let results = tokio::time::timeout(TIMEOUT, &mut pending)
        .await
        .expect("repository writes failed to drain after releasing the writer");
    for (name, result) in [
        "user", "manga", "chapter", "library", "history", "tracker", "download",
    ]
    .into_iter()
    .zip(results)
    {
        result.unwrap_or_else(|error| panic!("{name} writer failed: {error}"));
    }
    assert_eq!(
        UserRepositoryImpl::new(fixture.pool.clone())
            .get_user_by_id(1)
            .await
            .unwrap()
            .password,
        "updated"
    );
    assert_eq!(
        LibraryRepositoryImpl::new(fixture.pool.clone())
            .get_categories_by_user_id(1)
            .await
            .unwrap()[0]
            .name,
        "After rename"
    );
    let history_page: i64 = sqlx::query_scalar(
        "SELECT last_page FROM user_history WHERE user_id = 1 AND chapter_id = 1",
    )
    .fetch_one(fixture.pool.read())
    .await
    .unwrap();
    assert_eq!(history_page, 3);
    fixture.pool.close().await;
}

#[tokio::test]
async fn write_transactions_preserve_visibility_and_both_pools_close() {
    let fixture = Fixture::new().await;
    let mut tx = fixture.pool.write().begin().await.unwrap();
    sqlx::query("UPDATE user SET password = 'committed' WHERE id = 1")
        .execute(&mut *tx)
        .await
        .unwrap();
    let inside: String = sqlx::query_scalar("SELECT password FROM user WHERE id = 1")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(inside, "committed");
    let outside: String = sqlx::query_scalar("SELECT password FROM user WHERE id = 1")
        .fetch_one(fixture.pool.read())
        .await
        .unwrap();
    assert_eq!(outside, "original");
    tx.commit().await.unwrap();
    let committed: String = sqlx::query_scalar("SELECT password FROM user WHERE id = 1")
        .fetch_one(fixture.pool.read())
        .await
        .unwrap();
    assert_eq!(committed, "committed");

    fixture.pool.clone().close().await;
    assert!(fixture.pool.read().is_closed());
    assert!(fixture.pool.write().is_closed());
    let reopened = establish_connection(fixture.dir.join("test.db").to_str().unwrap(), false)
        .await
        .unwrap();
    let saved: String = sqlx::query_scalar("SELECT password FROM user WHERE id = 1")
        .fetch_one(reopened.read())
        .await
        .unwrap();
    assert_eq!(saved, "committed");
    reopened.close().await;
}
