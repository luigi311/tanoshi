//! Reproduce delayed web-page data while downloads are removed from a large queue.
//!
//! Normal functional tests use small queues suitable for debug builds and CI.
//!
//! Run the performance regression explicitly in a release build:
//! `cargo test -p tanoshi --release --test download_queue_responsiveness --locked
//! -- --ignored --nocapture --test-threads=1`
//!
//! Defaults: 5,000 chapters, 30 pages each, five simultaneous removals, three
//! samples, and a 100 ms limit for each page query. A second stress test mirrors
//! the console helper's five parallel batches cancelling 100 dummy chapters.
//! Tests use a temporary SQLite database with the production migrations, pool,
//! repositories, and GraphQL resolvers.
//! The opt-in stress tests verify that queue writers leave reader capacity
//! available and complete without database-lock errors.
//!
//! Optional environment variables:
//! - `TANOSHI_QUEUE_TEST_CHAPTERS`: queue size (at least six).
//! - `TANOSHI_QUEUE_TEST_PAGES`: pages per chapter; increase to amplify stalls.
//! - `TANOSHI_QUEUE_TEST_BUDGET_MS`: page-query response-time limit.
//! - `TANOSHI_QUEUE_TEST_REPORT`: write measurements as JSON to this path.
//!
//! Queries are included from tanoshi-schema so they stay identical to the web
//! app's requests. This tests server-side delays, not browser rendering or
//! connection scheduling. The download worker is stopped; cleanup commands are
//! retained in its channel. No source network requests or artificial DB locks
//! are used. The source-list query is a useful control because it reads cached
//! metadata rather than the shared database.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    future::Future,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_graphql::{Request, Response, Variables};
use chrono::Utc;
use futures::{Stream, StreamExt, future::join_all};
use serde::Serialize;
use serde_json::{Value, json};
use tanoshi::{
    application::worker::downloads,
    domain::{
        entities::download::DownloadQueue,
        repositories::download::DownloadRepository,
        services::{
            download::DownloadService, image::ImageService, library::LibraryService,
            source::SourceService, tracker::TrackerService, user::UserService,
        },
    },
    infrastructure::{
        auth::Claims,
        config::Config,
        database::{Pool, establish_connection},
        domain::repositories::{
            download::DownloadRepositoryImpl, history::HistoryRepositoryImpl,
            image::ImageRepositoryImpl, image_cache::ImageCacheRepositoryImpl,
            library::LibraryRepositoryImpl, manga::MangaRepositoryImpl,
            source::SourceRepositoryImpl, tracker::TrackerRepositoryImpl, user::UserRepositoryImpl,
        },
        local::Local,
    },
    presentation::graphql::schema::{DatabaseLoader, SchemaBuilder, TanoshiSchema},
};
use tanoshi_vm::prelude::{ExtensionManager, Source};
use tokio::{sync::Barrier, task::JoinSet, time::Instant};

const CONCURRENT_REMOVALS: usize = 5;
const SAMPLES: usize = 3;
const SOURCE_ID: i64 = 10_000;
const MANGA_TITLE: &str = "Queue regression manga";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const PAGE_QUERIES: [(&str, &str); 4] = [
    (
        "library_categories",
        include_str!("../../tanoshi-schema/graphql/fetch_categories.graphql"),
    ),
    (
        "library_manga",
        include_str!("../../tanoshi-schema/graphql/browse_favorites.graphql"),
    ),
    (
        "catalogue_sources",
        include_str!("../../tanoshi-schema/graphql/fetch_sources.graphql"),
    ),
    (
        "settings_me",
        include_str!("../../tanoshi-schema/graphql/fetch_me.graphql"),
    ),
];
const REMOVE_QUERY: &str =
    include_str!("../../tanoshi-schema/graphql/remove_chapter_from_queue.graphql");
const MOVE_QUERY: &str = include_str!("../../tanoshi-schema/graphql/move_chapter_in_queue.graphql");
const SUBSCRIBE_QUERY: &str =
    include_str!("../../tanoshi-schema/graphql/subscribe_download_queue.graphql");

struct Fixture {
    dir: PathBuf,
    pool: Pool,
    repo: DownloadRepositoryImpl,
    schema: TanoshiSchema,
    _download_receiver: tokio::sync::mpsc::UnboundedReceiver<downloads::Command>,
}

impl Fixture {
    async fn new(chapters: i64) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-queue-responsiveness-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = establish_connection(dir.join("test.db").to_str().unwrap(), true)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO user (id, username, password, is_admin) \
             VALUES (1, 'queue-test-admin', 'unused-test-password', true)",
        )
        .execute(pool.write())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO manga (id, source_id, title, author, genre, path, cover_url, date_added) \
             VALUES (1, ?, ?, '[]', '[]', '/manga', 'https://example.test/cover.jpg', CURRENT_TIMESTAMP)",
        )
        .bind(SOURCE_ID)
        .bind(MANGA_TITLE)
        .execute(pool.write())
        .await
        .unwrap();
        sqlx::query("INSERT INTO user_library (user_id, manga_id) VALUES (1, 1)")
            .execute(pool.write())
            .await
            .unwrap();
        sqlx::query(
            "WITH RECURSIVE ids(id) AS (SELECT 1 UNION ALL SELECT id + 1 FROM ids WHERE id < ?) \
             INSERT INTO chapter (id, source_id, manga_id, title, path, number, uploaded, date_added) \
             SELECT id, ?, 1, 'Chapter ' || id, '/chapter/' || id, id, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP \
             FROM ids",
        )
        .bind(chapters)
        .bind(SOURCE_ID)
        .execute(pool.write())
        .await
        .unwrap();

        let ext = ExtensionManager::new(dir.join("plugins"));
        ext.insert(Source::from(Box::new(Local::new(
            SOURCE_ID,
            "Queue regression source".into(),
            &dir,
        ))))
        .await
        .unwrap();

        let repo = DownloadRepositoryImpl::new(pool.clone());
        let library_repo = LibraryRepositoryImpl::new(pool.clone());
        let tracker_repo = TrackerRepositoryImpl::new(pool.clone(), None, None);
        let (download_sender, download_receiver) = downloads::channel();
        let mut config = Config::default();
        config.secret = "queue-test-key16".into();
        config.download_path = dir.to_string_lossy().into_owned();
        let schema = SchemaBuilder::new()
            .data(config)
            .data(Claims {
                sub: 1,
                username: "queue-test-admin".into(),
                is_admin: true,
                exp: usize::MAX,
            })
            .data(UserService::new(UserRepositoryImpl::new(pool.clone())))
            .data(TrackerService::new(tracker_repo.clone()))
            .data(SourceService::new(SourceRepositoryImpl::new(ext.clone())))
            .data(LibraryService::new(library_repo.clone()))
            .data(ImageService::new(
                ImageRepositoryImpl::new(ext.clone()),
                ImageCacheRepositoryImpl::new(dir.join("cache")),
            ))
            .data(DownloadService::new(repo.clone(), download_sender))
            .loader(DatabaseLoader::new(
                HistoryRepositoryImpl::new(pool.clone()),
                library_repo,
                MangaRepositoryImpl::new(pool.clone()),
                tracker_repo,
                repo.clone(),
            ))
            .data(ext)
            .build();
        Self {
            dir,
            pool,
            repo,
            schema,
            _download_receiver: download_receiver,
        }
    }

    async fn seed_downloaded_chapters(&self, chapters: i64) {
        sqlx::query(
            "UPDATE chapter SET uploaded = '2000-01-01 00:00:00', \
             date_added = '2000-01-01 00:00:00', \
             downloaded_path = CASE WHEN id <= ? THEN '/downloads/' || id || '.cbz' END",
        )
        .bind(chapters)
        .execute(self.pool.write())
        .await
        .unwrap();
    }

    async fn seed_queue(&self, chapters: i64, pages: i64) {
        sqlx::query("DELETE FROM download_queue")
            .execute(self.pool.write())
            .await
            .unwrap();
        let date_added = Utc::now().naive_utc();
        // Limit bound parameters even when the page count is increased.
        let mut batch = Vec::with_capacity(1_000);
        for chapter_id in 1..=chapters {
            for rank in 0..pages {
                batch.push(DownloadQueue {
                    id: 0,
                    source_id: SOURCE_ID,
                    source_name: "Queue regression source".into(),
                    manga_id: 1,
                    manga_title: MANGA_TITLE.into(),
                    chapter_id,
                    chapter_title: format!("Chapter {chapter_id}"),
                    rank,
                    url: format!("https://example.test/{chapter_id}/{rank}.jpg"),
                    priority: chapter_id,
                    date_added,
                });
                if batch.len() == 1_000 {
                    self.repo.insert_download_queue(&batch).await.unwrap();
                    batch.clear();
                }
            }
        }
        self.repo.insert_download_queue(&batch).await.unwrap();
    }

    async fn close(self) {
        self.pool.close().await;
        remove_fixture_dir(&self.dir)
            .await
            .unwrap_or_else(|error| panic!("failed to remove {}: {error}", self.dir.display()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

async fn remove_fixture_dir(dir: &Path) -> std::io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match fs::remove_dir_all(dir) {
            Ok(()) => return Ok(()),
            // Closing pools does not prevent transient Windows sharing locks
            // during teardown. Yield between attempts so pending work can finish.
            Err(error)
                if cfg!(windows)
                    && matches!(error.raw_os_error(), Some(32 | 33))
                    && Instant::now() < deadline =>
            {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(windows)]
#[tokio::test]
async fn fixture_cleanup_waits_for_windows_file_handles_to_close() {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

    let fixture = Fixture::new(1).await;
    fixture.pool.close().await;
    let path = fixture.dir.join("held-open.txt");
    fs::write(&path, b"cleanup regression").unwrap();
    let file = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    assert_eq!(fs::remove_file(&path).unwrap_err().raw_os_error(), Some(32));

    let mut cleanup = Box::pin(remove_fixture_dir(&fixture.dir));
    assert!(futures::poll!(cleanup.as_mut()).is_pending());
    assert!(path.exists(), "cleanup must wait for the file handle");
    drop(file);
    tokio::time::timeout(Duration::from_secs(5), cleanup)
        .await
        .expect("fixture cleanup did not finish after releasing the file")
        .unwrap();
    assert!(!fixture.dir.exists());
}

#[derive(Debug, Serialize)]
struct Measurement {
    query: &'static str,
    elapsed_ms: f64,
    data: Value,
}

async fn measure(schema: &TanoshiSchema, name: &'static str, request: Request) -> Measurement {
    let start = Instant::now();
    let response = tokio::time::timeout(REQUEST_TIMEOUT, schema.execute(request))
        .await
        .unwrap_or_else(|_| panic!("{name} timed out"));
    assert!(response.errors.is_empty(), "{name}: {:?}", response.errors);
    Measurement {
        query: name,
        elapsed_ms: start.elapsed().as_secs_f64() * 1_000.0,
        data: response.data.into_json().unwrap(),
    }
}

async fn page_requests(schema: &TanoshiSchema) -> Vec<Measurement> {
    join_all(PAGE_QUERIES.map(|(name, query)| {
        measure(
            schema,
            name,
            Request::new(query).variables(Variables::from_json(json!({"categoryId": null}))),
        )
    }))
    .await
}

fn assert_page_data(measurements: &[Measurement]) {
    assert_eq!(measurements[0].data["getCategories"][0]["count"], 1);
    assert_eq!(measurements[1].data["library"].as_array().unwrap().len(), 1);
    assert_eq!(measurements[1].data["library"][0]["title"], MANGA_TITLE);
    assert_eq!(
        measurements[1].data["library"][0]["source"]["id"],
        SOURCE_ID
    );
    assert_eq!(
        measurements[2].data["installedSources"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(measurements[2].data["installedSources"][0]["id"], SOURCE_ID);
    assert_eq!(measurements[3].data["me"]["isAdmin"], true);
}

fn positive_setting(name: &str, default: u64) -> u64 {
    let value = std::env::var(name)
        .map(|value| {
            value
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("invalid {name}"))
        })
        .unwrap_or(default);
    assert!(value > 0, "{name} must be positive");
    value
}

#[tokio::test]
async fn web_page_queries_return_existing_data() {
    let fixture = Fixture::new(10).await;
    fixture.seed_queue(10, 2).await;
    assert_page_data(&page_requests(&fixture.schema).await);
    fixture.close().await;
}

#[tokio::test]
async fn downloaded_chapters_without_limits_respect_cursor_bounds() {
    let fixture = Fixture::new(27).await;
    fixture.seed_downloaded_chapters(25).await;
    let chapters = fixture
        .repo
        .get_downloaded_chapters(Utc::now().timestamp(), 1, 0, 0)
        .await
        .unwrap();
    assert_eq!(
        chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<Vec<_>>(),
        (1..=25).rev().collect::<Vec<_>>()
    );

    let timestamp = chapters[0].date_added.and_utc().timestamp();
    let window = fixture
        .repo
        .get_downloaded_chapters(timestamp, 21, timestamp, 5)
        .await
        .unwrap();
    assert_eq!(
        window.iter().map(|chapter| chapter.id).collect::<Vec<_>>(),
        (6..=20).rev().collect::<Vec<_>>()
    );
    assert!(
        fixture
            .repo
            .get_downloaded_chapters(timestamp, 5, timestamp, 4)
            .await
            .unwrap()
            .is_empty()
    );
    fixture.close().await;
}

#[tokio::test]
async fn downloaded_chapters_graphql_supports_omitted_and_explicit_limits() {
    let fixture = Fixture::new(27).await;
    fixture.seed_downloaded_chapters(25).await;
    for (arguments, ids, previous, next) in [
        (
            "(first: 20)",
            (6..=25).rev().collect::<Vec<_>>(),
            false,
            true,
        ),
        (
            "(last: 20)",
            (1..=20).rev().collect::<Vec<_>>(),
            true,
            false,
        ),
        ("", (1..=25).rev().collect::<Vec<_>>(), false, false),
    ] {
        let data = execute_data(
            &fixture.schema,
            Request::new(format!(
                "{{ getDownloadedChapters{arguments} {{ edges {{ node {{ id }} }} \
                 pageInfo {{ hasPreviousPage hasNextPage }} }} }}"
            )),
        )
        .await;
        let connection = &data["getDownloadedChapters"];
        assert_eq!(
            connection["edges"]
                .as_array()
                .unwrap()
                .iter()
                .map(|edge| edge["node"]["id"].as_i64().unwrap())
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(connection["pageInfo"]["hasPreviousPage"], previous);
        assert_eq!(connection["pageInfo"]["hasNextPage"], next);
    }

    fixture.seed_downloaded_chapters(0).await;
    let data = execute_data(
        &fixture.schema,
        Request::new("{ getDownloadedChapters { edges { node { id } } } }"),
    )
    .await;
    assert_eq!(data["getDownloadedChapters"]["edges"], json!([]));
    fixture.close().await;
}

#[tokio::test]
async fn downloaded_chapters_graphql_reports_database_errors() {
    let fixture = Fixture::new(1).await;
    sqlx::query("DROP TABLE chapter")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    let response = fixture
        .schema
        .execute("{ getDownloadedChapters { edges { node { id } } } }")
        .await;
    assert_eq!(response.errors.len(), 1, "{response:?}");
    assert!(
        response.errors[0]
            .message
            .contains("no such table: chapter")
    );
    fixture.close().await;
}

const SEED_QUERY: &str = "mutation Seed($runId: String!, $chapters: Int!, $pages: Int!) { \
    seedDownloadQueueRepro(runId: $runId, chapters: $chapters, pagesPerChapter: $pages) }";
const CLEAR_QUERY: &str =
    "mutation Clear($runId: String!) { clearDownloadQueueRepro(runId: $runId) }";
const REPRO_RUN: &str = "0123456789abcdef0123456789abcdef";

fn seed_request(run_id: &str, chapters: i64, pages: i64) -> Request {
    Request::new(SEED_QUERY).variables(Variables::from_json(json!({
        "runId": run_id, "chapters": chapters, "pages": pages,
    })))
}

fn clear_request(run_id: &str) -> Request {
    Request::new(CLEAR_QUERY).variables(Variables::from_json(json!({"runId": run_id})))
}

async fn execute_data(schema: &TanoshiSchema, request: Request) -> Value {
    let response = schema.execute(request).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

async fn next_queue_update(stream: &mut (impl Stream<Item = Response> + Unpin)) -> Value {
    let response = tokio::time::timeout(Duration::from_secs(10), stream.next())
        .await
        .expect("queue subscription timed out")
        .expect("queue subscription ended");
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap()["downloadQueueUpdates"].clone();
    assert_eq!(data["resyncRequired"], false);
    data
}

async fn queue_updates_through(
    stream: &mut (impl Stream<Item = Response> + Unpin),
    version: i64,
) -> Vec<Value> {
    let mut updates = vec![];
    loop {
        let update = next_queue_update(stream).await;
        assert_eq!(update["snapshot"], false);
        let current = update["version"].as_i64().unwrap();
        assert!(current <= version, "unexpected queue version: {update}");
        updates.push(update);
        if current == version {
            return updates;
        }
    }
}

fn apply_queue_updates(rows: &mut BTreeMap<i64, Value>, updates: &[Value]) {
    for update in updates {
        if update["snapshot"] == true {
            rows.clear();
        }
        for id in update["removedIds"].as_array().unwrap() {
            rows.remove(&id.as_i64().unwrap());
        }
        for row in update["updates"].as_array().unwrap() {
            rows.insert(row["chapterId"].as_i64().unwrap(), row.clone());
        }
    }
}

#[tokio::test]
async fn queue_subscribers_share_batches_and_accept_snapshots_between_changes() {
    let fixture = Fixture::new(4).await;
    fixture.seed_queue(4, 3).await;
    let mut first = Box::pin(fixture.schema.execute_stream(SUBSCRIBE_QUERY));
    let initial = next_queue_update(&mut first).await;
    assert_eq!(initial["snapshot"], true);
    assert_eq!(initial["version"], 0);
    assert_eq!(initial["updates"].as_array().unwrap().len(), 4);
    assert!(initial["updates"][0]["dateAdded"].as_i64().unwrap() > 0);

    let page_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM download_queue WHERE chapter_id = 1 ORDER BY rank")
            .fetch_all(fixture.pool.read())
            .await
            .unwrap();
    for id in &page_ids {
        fixture
            .repo
            .mark_single_download_queue_as_completed(*id)
            .await
            .unwrap();
    }
    // Retries and missing rows must not advance the version.
    fixture
        .repo
        .mark_single_download_queue_as_completed(page_ids[0])
        .await
        .unwrap();
    fixture
        .repo
        .delete_single_chapter_download_queue(999)
        .await
        .unwrap();

    let mut second = Box::pin(fixture.schema.execute_stream(SUBSCRIBE_QUERY));
    let later = next_queue_update(&mut second).await;
    assert_eq!(later["snapshot"], true);
    assert_eq!(later["version"], 3);
    assert_eq!(later["updates"][0]["downloaded"], 3);

    execute_data(
        &fixture.schema,
        Request::new(MOVE_QUERY).variables(Variables::from_json(json!({"id": 2, "up": true}))),
    )
    .await;
    execute_data(
        &fixture.schema,
        Request::new(REMOVE_QUERY).variables(Variables::from_json(json!({"ids": [3]}))),
    )
    .await;
    let first_batches = queue_updates_through(&mut first, 5).await;
    let second_batches = queue_updates_through(&mut second, 5).await;
    assert_eq!(
        first_batches.last(),
        second_batches.last(),
        "viewers must share the same batch"
    );
    for batch in first_batches.iter().chain(&second_batches) {
        assert!(
            batch["updates"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| { [1, 2].contains(&row["chapterId"].as_i64().unwrap()) }),
            "unchanged chapters were republished: {batch}"
        );
    }
    let mut first_rows = BTreeMap::new();
    let mut second_rows = BTreeMap::new();
    apply_queue_updates(&mut first_rows, &[initial]);
    apply_queue_updates(&mut first_rows, &first_batches);
    apply_queue_updates(&mut second_rows, &[later]);
    apply_queue_updates(&mut second_rows, &second_batches);
    assert_eq!(first_rows, second_rows);
    assert_eq!(first_rows.keys().copied().collect::<Vec<_>>(), [1, 2, 4]);
    assert_eq!(first_rows[&1]["downloaded"], 3);
    assert_eq!(first_rows[&1]["priority"], 2);
    assert_eq!(first_rows[&2]["priority"], 1);

    fixture
        .repo
        .reset_chapter_download_progress(1)
        .await
        .unwrap();
    let reset = queue_updates_through(&mut first, 6).await;
    assert_eq!(reset.last().unwrap()["updates"][0]["downloaded"], 0);
    // Archive completion uses this path instead of the cancellation mutation.
    fixture
        .repo
        .delete_single_chapter_download_queue(1)
        .await
        .unwrap();
    let completed = queue_updates_through(&mut first, 7).await;
    assert_eq!(completed.last().unwrap()["removedIds"], json!([1]));
    drop(first);
    drop(second);
    fixture.close().await;
}

#[tokio::test]
async fn queue_version_resets_only_when_the_last_viewer_leaves() {
    let fixture = Fixture::new(3).await;
    fixture.seed_queue(3, 1).await;
    let mut first = Box::pin(fixture.schema.execute_stream(SUBSCRIBE_QUERY));
    let mut second = Box::pin(fixture.schema.execute_stream(SUBSCRIBE_QUERY));
    assert_eq!(next_queue_update(&mut first).await["version"], 0);
    assert_eq!(next_queue_update(&mut second).await["version"], 0);
    fixture
        .repo
        .delete_single_chapter_download_queue(1)
        .await
        .unwrap();
    assert_eq!(
        queue_updates_through(&mut first, 1).await.last().unwrap()["version"],
        1
    );
    assert_eq!(
        queue_updates_through(&mut second, 1).await.last().unwrap()["version"],
        1
    );
    drop(first);
    let page = fixture
        .repo
        .get_single_download_queue()
        .await
        .unwrap()
        .unwrap();
    fixture
        .repo
        .mark_single_download_queue_as_completed(page.id)
        .await
        .unwrap();
    assert_eq!(
        queue_updates_through(&mut second, 2).await.last().unwrap()["updates"][0]["downloaded"],
        1
    );
    drop(second);

    // Mutations without viewers are already included in the next snapshot.
    fixture
        .repo
        .delete_single_chapter_download_queue(2)
        .await
        .unwrap();
    let mut fresh = Box::pin(fixture.schema.execute_stream(SUBSCRIBE_QUERY));
    let snapshot = next_queue_update(&mut fresh).await;
    assert_eq!(snapshot["snapshot"], true);
    assert_eq!(snapshot["version"], 0);
    assert_eq!(snapshot["updates"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["updates"][0]["chapterId"], 3);
    drop(fresh);
    fixture.close().await;
}

#[tokio::test]
async fn queue_subscription_includes_repro_changes_and_download_status() {
    let fixture = Fixture::new(1).await;
    fixture.seed_queue(1, 1).await;
    fs::write(fixture.dir.join(".pause"), b"").unwrap();
    let mut stream = Box::pin(fixture.schema.execute_stream(SUBSCRIBE_QUERY));
    assert_eq!(
        next_queue_update(&mut stream).await["downloadStatus"],
        false
    );
    let seeded = execute_data(&fixture.schema, seed_request(REPRO_RUN, 3, 2)).await;
    assert_eq!(
        execute_data(&fixture.schema, seed_request(REPRO_RUN, 3, 2)).await,
        seeded
    );
    let batches = queue_updates_through(&mut stream, 1).await;
    let ids: BTreeSet<i64> = seeded["seedDownloadQueueRepro"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_i64().unwrap())
        .collect();
    assert_eq!(
        batches.last().unwrap()["updates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["chapterId"].as_i64().unwrap())
            .collect::<BTreeSet<_>>(),
        ids
    );
    assert_eq!(
        execute_data(&fixture.schema, clear_request(REPRO_RUN)).await["clearDownloadQueueRepro"],
        3
    );
    execute_data(&fixture.schema, Request::new("mutation { resumeDownload }")).await;
    let cleared = queue_updates_through(&mut stream, 3).await;
    let removed: BTreeSet<_> = cleared
        .iter()
        .flat_map(|batch| {
            batch["removedIds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_i64().unwrap())
        })
        .collect();
    assert_eq!(removed, ids);
    assert!(
        cleared
            .iter()
            .all(|batch| batch["updates"].as_array().unwrap().is_empty())
    );
    assert_eq!(cleared.last().unwrap()["downloadStatus"], true);
    // Pause alone publishes status without aggregating every queue chapter.
    execute_data(&fixture.schema, Request::new("mutation { pauseDownload }")).await;
    let paused = queue_updates_through(&mut stream, 4).await;
    assert_eq!(paused.last().unwrap()["downloadStatus"], false);
    assert!(
        paused.last().unwrap()["updates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(stream);
    fixture.close().await;
}

#[tokio::test]
async fn queue_subscription_requires_an_administrator() {
    let fixture = Fixture::new(1).await;
    let mut denied = Box::pin(
        fixture
            .schema
            .execute_stream(Request::new(SUBSCRIBE_QUERY).data(Claims {
                sub: 2,
                username: "reader".into(),
                is_admin: false,
                exp: usize::MAX,
            })),
    );
    let response = denied.next().await.unwrap();
    assert!(
        response
            .errors
            .iter()
            .any(|error| error.message == "Forbidden")
    );
    drop(denied);
    let anonymous = SchemaBuilder::new().build();
    assert!(
        !anonymous
            .execute_stream(SUBSCRIBE_QUERY)
            .next()
            .await
            .unwrap()
            .errors
            .is_empty()
    );
    fixture.close().await;
}

#[tokio::test]
async fn queue_snapshot_waiting_for_the_writer_leaves_readers_available() {
    let fixture = Fixture::new(2).await;
    fixture.seed_queue(2, 1).await;
    let writer = fixture.pool.write().acquire().await.unwrap();
    let repo = fixture.repo.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let snapshot = tokio::spawn(async move {
        let mut loading = Box::pin(repo.subscribe_download_queue());
        let mut started = Some(started);
        futures::future::poll_fn(|cx| {
            let result = loading.as_mut().poll(cx);
            if let Some(started) = started.take() {
                assert!(result.is_pending());
                let _ = started.send(());
            }
            result
        })
        .await
        .unwrap()
    });
    ready.await.unwrap();
    let repo = fixture.repo.clone();
    let removal =
        tokio::spawn(async move { repo.delete_single_chapter_download_queue(1).await.unwrap() });
    let pages = tokio::time::timeout(Duration::from_secs(5), page_requests(&fixture.schema))
        .await
        .expect("queue synchronization blocked ordinary reader requests");
    assert_page_data(&pages);
    drop(writer);
    let mut stream = Box::pin(snapshot.await.unwrap());
    let initial = stream.next().await.unwrap();
    assert!(initial.snapshot);
    removal.await.unwrap();
    // Either order is legal, but the snapshot's rows and version must agree.
    if initial.version == 0 {
        assert_eq!(initial.updates.len(), 2);
        let batch = tokio::time::timeout(Duration::from_secs(10), stream.next())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(batch.from_version, 0);
        assert_eq!(batch.version, 1);
        assert_eq!(batch.removed_ids, [1]);
    } else {
        assert_eq!(initial.version, 1);
        assert_eq!(initial.updates.len(), 1);
        assert_eq!(initial.updates[0].chapter_id, 2);
    }
    drop(stream);
    fixture.close().await;
}

async fn real_queue_snapshot(pool: &Pool) -> String {
    sqlx::query_scalar(
        "SELECT json_group_array(json_object('id', id, 'chapter', chapter_id, 'url', url, \
         'priority', priority, 'downloaded', downloaded)) \
         FROM (SELECT * FROM download_queue WHERE chapter_id > 0 ORDER BY id)",
    )
    .fetch_one(pool.read())
    .await
    .unwrap()
}

#[tokio::test]
async fn queue_cancellation_preserves_remaining_priorities_and_pages() {
    let fixture = Fixture::new(6).await;
    fixture.seed_queue(6, 3).await;
    let before: Vec<Value> =
        serde_json::from_str(&real_queue_snapshot(&fixture.pool).await).unwrap();
    // Catch priority rewrites even if they would later restore the same values.
    sqlx::query(
        "CREATE TRIGGER no_cancellation_renumber BEFORE UPDATE OF priority ON download_queue \
         BEGIN SELECT RAISE(ABORT, 'cancellation must preserve priorities'); END",
    )
    .execute(fixture.pool.write())
    .await
    .unwrap();

    let result = execute_data(
        &fixture.schema,
        Request::new(REMOVE_QUERY).variables(Variables::from_json(json!({"ids": [2, 4, 2, 999]}))),
    )
    .await;
    assert_eq!(result["removeChaptersFromQueue"], 4);
    let after: Vec<Value> =
        serde_json::from_str(&real_queue_snapshot(&fixture.pool).await).unwrap();
    let expected: Vec<Value> = before
        .into_iter()
        .filter(|page| page["chapter"] != 2 && page["chapter"] != 4)
        .collect();
    assert_eq!(after, expected);
    let queue = fixture.repo.get_download_queue(&[]).await.unwrap();
    assert_eq!(
        queue
            .iter()
            .map(|chapter| (chapter.chapter_id, chapter.priority))
            .collect::<Vec<_>>(),
        [(1, 1), (3, 3), (5, 5), (6, 6)],
    );
    assert_eq!(
        fixture
            .repo
            .get_download_queue_last_priority()
            .await
            .unwrap(),
        Some(6)
    );
    fixture.close().await;
}

#[tokio::test]
async fn queue_reordering_swaps_only_neighbouring_chapters_across_gaps() {
    let fixture = Fixture::new(6).await;
    fixture.seed_queue(6, 3).await;
    // The worker starts an empty queue at priority zero.
    sqlx::query("UPDATE download_queue SET priority = 0 WHERE chapter_id = 1")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    // Reverse insertion times so selecting by date instead of priority fails.
    sqlx::query("UPDATE download_queue SET date_added = 1000 - chapter_id")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    execute_data(
        &fixture.schema,
        Request::new(REMOVE_QUERY).variables(Variables::from_json(json!({"ids": [2, 4]}))),
    )
    .await;
    sqlx::query("UPDATE download_queue SET downloaded = true WHERE chapter_id = 3 AND rank = 0")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    sqlx::query("CREATE TABLE priority_updates (chapter_id INTEGER NOT NULL)")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER record_priority_update AFTER UPDATE OF priority ON download_queue \
         BEGIN INSERT INTO priority_updates VALUES (NEW.chapter_id); END",
    )
    .execute(fixture.pool.write())
    .await
    .unwrap();

    // Exercise both directions at the head and tail. The request supplies only
    // a direction; the expected neighbour comes from the database's priorities.
    for (chapter_id, up, priority, neighbour_id, order) in [
        (3, true, 0, 1, [3, 1, 5, 6]),
        (3, false, 3, 1, [1, 3, 5, 6]),
        (5, false, 6, 6, [1, 3, 6, 5]),
        (5, true, 5, 6, [1, 3, 5, 6]),
    ] {
        let mut expected: Vec<Value> =
            serde_json::from_str(&real_queue_snapshot(&fixture.pool).await).unwrap();
        let previous = expected
            .iter()
            .find(|page| page["chapter"] == chapter_id)
            .unwrap()["priority"]
            .clone();
        for page in &mut expected {
            if page["chapter"] == chapter_id {
                page["priority"] = json!(priority);
            } else if page["chapter"] == neighbour_id {
                page["priority"] = previous.clone();
            }
        }
        let result = execute_data(
            &fixture.schema,
            Request::new(MOVE_QUERY).variables(Variables::from_json(json!({
                "id": chapter_id, "up": up,
            }))),
        )
        .await;
        assert_eq!(result["moveChapterInQueue"], true);
        let after: Vec<Value> =
            serde_json::from_str(&real_queue_snapshot(&fixture.pool).await).unwrap();
        assert_eq!(after, expected, "page rows or unrelated priorities changed");
        let queue = fixture.repo.get_download_queue(&[]).await.unwrap();
        assert_eq!(
            queue
                .iter()
                .map(|chapter| chapter.chapter_id)
                .collect::<Vec<_>>(),
            order
        );
        assert_eq!(
            queue
                .iter()
                .find(|chapter| chapter.chapter_id == 3)
                .unwrap()
                .downloaded,
            1
        );
        assert_eq!(
            fixture
                .repo
                .get_single_download_queue()
                .await
                .unwrap()
                .unwrap()
                .chapter_id,
            order[0]
        );
        let changed: Vec<i64> =
            sqlx::query_scalar("SELECT chapter_id FROM priority_updates ORDER BY chapter_id")
                .fetch_all(fixture.pool.read())
                .await
                .unwrap();
        let mut expected_changes = vec![chapter_id; 3];
        expected_changes.extend([neighbour_id; 3]);
        expected_changes.sort_unstable();
        assert_eq!(
            changed, expected_changes,
            "a move must update only two chapters"
        );
        sqlx::query("DELETE FROM priority_updates")
            .execute(fixture.pool.write())
            .await
            .unwrap();
    }
    fixture.close().await;
}

#[tokio::test]
async fn queue_reordering_ignores_missing_chapters_and_queue_boundaries() {
    let fixture = Fixture::new(3).await;
    fixture.seed_queue(3, 2).await;
    execute_data(
        &fixture.schema,
        Request::new(REMOVE_QUERY).variables(Variables::from_json(json!({"ids": [2]}))),
    )
    .await;
    let before = real_queue_snapshot(&fixture.pool).await;
    for (id, up) in [(999, true), (999, false), (1, true), (3, false)] {
        execute_data(
            &fixture.schema,
            Request::new(MOVE_QUERY).variables(Variables::from_json(json!({
                "id": id, "up": up,
            }))),
        )
        .await;
        assert_eq!(real_queue_snapshot(&fixture.pool).await, before);
    }
    execute_data(
        &fixture.schema,
        Request::new(REMOVE_QUERY).variables(Variables::from_json(json!({"ids": [1, 3]}))),
    )
    .await;
    for up in [true, false] {
        execute_data(
            &fixture.schema,
            Request::new(MOVE_QUERY).variables(Variables::from_json(json!({"id": 1, "up": up}))),
        )
        .await;
    }
    assert!(
        fixture
            .repo
            .get_download_queue(&[])
            .await
            .unwrap()
            .is_empty()
    );
    fixture.close().await;
}

#[tokio::test]
async fn queue_reordering_uses_current_neighbour_during_cancellation() {
    let fixture = Fixture::new(4).await;
    fixture.seed_queue(4, 2).await;
    join_all([
        execute_data(
            &fixture.schema,
            Request::new(REMOVE_QUERY).variables(Variables::from_json(json!({"ids": [2]}))),
        ),
        execute_data(
            &fixture.schema,
            Request::new(MOVE_QUERY).variables(Variables::from_json(json!({
                "id": 3, "up": true,
            }))),
        ),
    ])
    .await;
    let queue = fixture.repo.get_download_queue(&[]).await.unwrap();
    let order = queue
        .iter()
        .map(|chapter| (chapter.chapter_id, chapter.priority))
        .collect::<Vec<_>>();
    // If cancellation wins, chapter 1 becomes the current neighbour. Otherwise
    // chapter 3 trades priorities with chapter 2 before chapter 2 is removed.
    assert!(
        [vec![(3, 1), (1, 3), (4, 4)], vec![(1, 1), (3, 2), (4, 4)]].contains(&order),
        "move did not use the current neighbour: {order:?}",
    );
    assert!(queue.iter().all(|chapter| chapter.total == 2));
    fixture.close().await;
}

#[tokio::test]
async fn queue_reordering_applies_each_move_to_the_latest_order() {
    let fixture = Fixture::new(4).await;
    fixture.seed_queue(4, 2).await;
    join_all((0..2).map(|_| {
        execute_data(
            &fixture.schema,
            Request::new(MOVE_QUERY).variables(Variables::from_json(json!({
                "id": 3, "up": true,
            }))),
        )
    }))
    .await;
    let queue = fixture.repo.get_download_queue(&[]).await.unwrap();
    assert_eq!(
        queue
            .iter()
            .map(|chapter| (chapter.chapter_id, chapter.priority))
            .collect::<Vec<_>>(),
        [(3, 1), (1, 2), (2, 3), (4, 4)],
    );
    assert!(queue.iter().all(|chapter| chapter.total == 2));
    fixture.close().await;
}

#[tokio::test]
async fn queue_repro_seeds_and_cancels_only_dummy_entries() {
    // Check isolation and cleanup without a stress-sized debug-build workload.
    // The opt-in release stress test below covers 5,000 chapters with 30 pages.
    const CHAPTERS: i64 = 25;
    const PAGES: i64 = 3;
    let fixture = Fixture::new(3).await;
    fixture.seed_queue(3, 2).await;
    fs::write(fixture.dir.join(".pause"), b"").unwrap();
    let before = real_queue_snapshot(&fixture.pool).await;
    let data = execute_data(&fixture.schema, seed_request(REPRO_RUN, CHAPTERS, PAGES)).await;
    let ids: Vec<i64> = data["seedDownloadQueueRepro"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_i64().unwrap())
        .collect();
    assert_eq!(ids.len(), CHAPTERS as usize);
    assert!(ids.iter().all(|id| *id < 0));
    assert_eq!(real_queue_snapshot(&fixture.pool).await, before);
    let queue = fixture.repo.get_download_queue(&ids).await.unwrap();
    assert_eq!(queue.len(), CHAPTERS as usize);
    assert!(
        queue
            .iter()
            .all(|chapter| chapter.total == PAGES && chapter.priority > 3)
    );
    // Dummy entries participate in the same worker lookup and cancellation path.
    let normal = fixture
        .repo
        .get_single_download_queue()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(normal.chapter_id, 1);

    let repeated = execute_data(&fixture.schema, seed_request(REPRO_RUN, CHAPTERS, PAGES)).await;
    assert_eq!(repeated, data, "a retry must not seed another batch");
    assert_eq!(
        fixture.repo.get_download_queue(&[]).await.unwrap().len(),
        CHAPTERS as usize + 3
    );
    let other_run = "fedcba9876543210fedcba9876543210";
    let other = execute_data(&fixture.schema, seed_request(other_run, 7, 2)).await;
    let other_ids: Vec<i64> = other["seedDownloadQueueRepro"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_i64().unwrap())
        .collect();
    assert!(other_ids.iter().all(|id| !ids.contains(id)));

    let cancellations = join_all(ids[..CONCURRENT_REMOVALS].iter().map(|id| {
        execute_data(
            &fixture.schema,
            Request::new(REMOVE_QUERY).variables(Variables::from_json(json!({"ids": [id]}))),
        )
    }))
    .await;
    assert!(
        cancellations
            .iter()
            .all(|data| data["removeChaptersFromQueue"] == 1)
    );
    let remaining = CHAPTERS - CONCURRENT_REMOVALS as i64;
    assert_eq!(
        fixture.repo.get_download_queue(&ids).await.unwrap().len(),
        remaining as usize
    );
    assert_eq!(real_queue_snapshot(&fixture.pool).await, before);

    let cleared = execute_data(&fixture.schema, clear_request(REPRO_RUN)).await;
    assert_eq!(cleared["clearDownloadQueueRepro"], remaining);
    assert!(
        fixture
            .repo
            .get_download_queue(&ids)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fixture
            .repo
            .get_download_queue(&other_ids)
            .await
            .unwrap()
            .len(),
        7
    );
    assert_eq!(real_queue_snapshot(&fixture.pool).await, before);
    assert_eq!(
        execute_data(&fixture.schema, clear_request(REPRO_RUN)).await["clearDownloadQueueRepro"],
        0
    );
    assert_eq!(
        execute_data(&fixture.schema, clear_request(other_run)).await["clearDownloadQueueRepro"],
        7
    );
    assert_eq!(real_queue_snapshot(&fixture.pool).await, before);
    // No fabricated manga or chapter records are left in the user's collection.
    let chapters: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chapter")
        .fetch_one(fixture.pool.read())
        .await
        .unwrap();
    assert_eq!(chapters, 3);
    assert_page_data(&page_requests(&fixture.schema).await);
    fixture.close().await;
}

#[tokio::test]
async fn queue_repro_requires_admin_pause_and_valid_parameters() {
    let fixture = Fixture::new(1).await;
    fixture.seed_queue(1, 1).await;
    let before = real_queue_snapshot(&fixture.pool).await;
    let running = fixture.schema.execute(seed_request(REPRO_RUN, 10, 2)).await;
    assert!(
        running
            .errors
            .iter()
            .any(|error| error.message.contains("Pause downloads"))
    );
    fs::write(fixture.dir.join(".pause"), b"").unwrap();

    for request in [seed_request(REPRO_RUN, 10, 2), clear_request(REPRO_RUN)] {
        let denied = fixture
            .schema
            .execute(request.data(Claims {
                sub: 2,
                username: "reader".into(),
                is_admin: false,
                exp: usize::MAX,
            }))
            .await;
        assert!(
            denied
                .errors
                .iter()
                .any(|error| error.message == "Forbidden")
        );
    }
    let anonymous = SchemaBuilder::new().build();
    for request in [seed_request(REPRO_RUN, 10, 2), clear_request(REPRO_RUN)] {
        assert!(!anonymous.execute(request).await.errors.is_empty());
    }
    for (chapters, pages) in [(0, 1), (5_001, 1), (1, 0), (1, 101)] {
        assert!(
            !fixture
                .schema
                .execute(seed_request(REPRO_RUN, chapters, pages))
                .await
                .errors
                .is_empty()
        );
    }
    assert!(
        !fixture
            .schema
            .execute(seed_request("invalid", 1, 1))
            .await
            .errors
            .is_empty()
    );
    assert!(
        !fixture
            .schema
            .execute(clear_request("invalid"))
            .await
            .errors
            .is_empty()
    );
    assert_eq!(fixture.repo.get_download_queue(&[]).await.unwrap().len(), 1);
    assert_eq!(real_queue_snapshot(&fixture.pool).await, before);
    fixture.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "opt-in release stress test: five parallel batches cancelling 100 of 5000 chapters"]
async fn queue_repro_bulk_cancellations_complete_without_database_lock_errors() {
    let fixture = Fixture::new(3).await;
    fixture.seed_queue(3, 2).await;
    fs::write(fixture.dir.join(".pause"), b"").unwrap();
    let before = real_queue_snapshot(&fixture.pool).await;
    let seeded = execute_data(&fixture.schema, seed_request(REPRO_RUN, 5_000, 30)).await;
    let ids: Vec<i64> = seeded["seedDownloadQueueRepro"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_i64().unwrap())
        .collect();
    let mut viewer = Box::pin(fixture.schema.execute_stream(SUBSCRIBE_QUERY));
    let snapshot = next_queue_update(&mut viewer).await;
    assert_eq!(snapshot["updates"].as_array().unwrap().len(), 5_003);
    let snapshot_bytes = serde_json::to_vec(&snapshot).unwrap().len();
    let barrier = Arc::new(Barrier::new(CONCURRENT_REMOVALS + 1));
    let active = Arc::new(AtomicUsize::new(CONCURRENT_REMOVALS));
    let mut removals = JoinSet::new();
    let started = Instant::now();
    for group in 0..CONCURRENT_REMOVALS {
        let selected: Vec<i64> = ids[..100]
            .iter()
            .skip(group)
            .step_by(CONCURRENT_REMOVALS)
            .copied()
            .collect();
        let schema = fixture.schema.clone();
        let barrier = barrier.clone();
        let active = active.clone();
        removals.spawn(async move {
            barrier.wait().await;
            let response = tokio::time::timeout(
                Duration::from_secs(120),
                schema.execute(
                    Request::new(REMOVE_QUERY)
                        .variables(Variables::from_json(json!({"ids": selected}))),
                ),
            )
            .await
            .unwrap();
            active.fetch_sub(1, Ordering::SeqCst);
            response
        });
    }
    barrier.wait().await;
    let mut page_samples = Vec::new();
    while active.load(Ordering::SeqCst) > 0 && started.elapsed() < Duration::from_secs(120) {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let pages = page_requests(&fixture.schema).await;
        assert_page_data(&pages);
        page_samples.push(pages);
    }
    let mut responses = Vec::new();
    while let Some(result) = removals.join_next().await {
        responses.push(result.unwrap());
    }
    let duration = started.elapsed();
    let failures: Vec<_> = responses
        .iter()
        .flat_map(|response| &response.errors)
        .collect();
    let maximum_page_ms = page_samples
        .iter()
        .flatten()
        .map(|page| page.elapsed_ms)
        .fold(0.0_f64, f64::max);
    println!(
        "100 cancellations: {:.2}s, {} failed requests, max page query {:.2}ms",
        duration.as_secs_f64(),
        failures.len(),
        maximum_page_ms,
    );
    assert!(failures.is_empty(), "{failures:?}");
    for response in responses {
        assert_eq!(
            response.data.into_json().unwrap()["removeChaptersFromQueue"],
            20
        );
    }
    let batches = queue_updates_through(&mut viewer, 100).await;
    let removed: BTreeSet<_> = batches
        .iter()
        .flat_map(|batch| {
            batch["removedIds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_i64().unwrap())
        })
        .collect();
    assert_eq!(removed, ids[..100].iter().copied().collect());
    assert!(
        batches
            .iter()
            .all(|batch| batch["updates"].as_array().unwrap().is_empty()),
        "cancellation republished unchanged chapters"
    );
    let batch_bytes = serde_json::to_vec(&batches).unwrap().len();
    println!(
        "queue subscription: {snapshot_bytes} snapshot bytes, {batch_bytes} bytes across {} cancellation batches",
        batches.len()
    );
    drop(viewer);
    let remaining = fixture.repo.get_download_queue(&ids).await.unwrap();
    assert_eq!(remaining.len(), 4_900);
    assert!(
        remaining
            .iter()
            .all(|chapter| !ids[..100].contains(&chapter.chapter_id))
    );
    assert_eq!(real_queue_snapshot(&fixture.pool).await, before);
    assert_eq!(
        execute_data(&fixture.schema, clear_request(REPRO_RUN)).await["clearDownloadQueueRepro"],
        4_900
    );
    assert_eq!(real_queue_snapshot(&fixture.pool).await, before);
    fixture.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "opt-in release performance regression"]
async fn web_page_requests_stay_responsive_during_queue_removals() {
    let chapters = i64::try_from(positive_setting("TANOSHI_QUEUE_TEST_CHAPTERS", 5_000)).unwrap();
    let pages = i64::try_from(positive_setting("TANOSHI_QUEUE_TEST_PAGES", 30)).unwrap();
    let budget_ms = positive_setting("TANOSHI_QUEUE_TEST_BUDGET_MS", 100) as f64;
    assert!(chapters > CONCURRENT_REMOVALS as i64);
    let fixture = Fixture::new(chapters).await;
    fixture.seed_queue(chapters, pages).await;
    let baseline = page_requests(&fixture.schema).await;
    assert_page_data(&baseline);
    let mut samples = Vec::new();

    for sample in 1..=SAMPLES {
        if sample > 1 {
            fixture.seed_queue(chapters, pages).await;
        }
        let barrier = Arc::new(Barrier::new(CONCURRENT_REMOVALS + 1));
        let active = Arc::new(AtomicUsize::new(CONCURRENT_REMOVALS));
        let mut removals = JoinSet::new();
        for chapter_id in 1..=CONCURRENT_REMOVALS as i64 {
            let schema = fixture.schema.clone();
            let barrier = barrier.clone();
            let active = active.clone();
            removals.spawn(async move {
                barrier.wait().await;
                let result = measure(
                    &schema,
                    "remove_chapter",
                    Request::new(REMOVE_QUERY)
                        .variables(Variables::from_json(json!({"ids": [chapter_id]}))),
                )
                .await;
                active.fetch_sub(1, Ordering::SeqCst);
                assert_eq!(result.data["removeChaptersFromQueue"], 1);
                result
            });
        }
        barrier.wait().await;

        // Observe real pool pressure rather than creating locks in the test.
        // Fast removals can finish before it develops, which is a valid fix.
        let wait_start = Instant::now();
        let mut busy_since = None;
        while active.load(Ordering::SeqCst) > 0 && wait_start.elapsed() < Duration::from_millis(250)
        {
            if fixture.pool.write().size() == fixture.pool.write().options().get_max_connections()
                && fixture.pool.write().num_idle() == 0
            {
                let since = busy_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_millis(10) {
                    break;
                }
            } else {
                busy_since = None;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        let active_at_probe = active.load(Ordering::SeqCst);
        let loaded = page_requests(&fixture.schema).await;
        assert_page_data(&loaded);
        for (before, during) in baseline.iter().zip(&loaded) {
            assert_eq!(before.data, during.data, "{} data changed", during.query);
        }
        let mut completed = Vec::new();
        while let Some(result) = removals.join_next().await {
            completed.push(result.unwrap());
        }
        let remaining = fixture.repo.get_download_queue(&[]).await.unwrap();
        assert_eq!(
            remaining
                .iter()
                .map(|entry| entry.chapter_id)
                .collect::<Vec<_>>(),
            (CONCURRENT_REMOVALS as i64 + 1..=chapters).collect::<Vec<_>>()
        );
        assert!(remaining.iter().all(|entry| entry.total == pages));
        let result = json!({
            "sample": sample,
            "active_removals_at_probe": active_at_probe,
            "page_requests": loaded,
            "removals": completed,
        });
        println!("QUEUE_RESPONSIVENESS_SAMPLE {result}");
        samples.push(result);
    }

    assert_page_data(&page_requests(&fixture.schema).await);
    let violations = samples
        .iter()
        .flat_map(|sample| sample["page_requests"].as_array().unwrap())
        .filter(|result| result["elapsed_ms"].as_f64().unwrap() > budget_ms)
        .map(|result| {
            format!(
                "{}: {:.1} ms",
                result["query"],
                result["elapsed_ms"].as_f64().unwrap()
            )
        })
        .collect::<Vec<_>>();
    let report = json!({
        "chapters": chapters,
        "pages_per_chapter": pages,
        "concurrent_removals": CONCURRENT_REMOVALS,
        "page_request_budget_ms": budget_ms,
        "baseline": baseline,
        "samples": samples,
        "violations": violations,
    });
    println!("QUEUE_RESPONSIVENESS_REPORT {report}");
    if let Ok(path) = std::env::var("TANOSHI_QUEUE_TEST_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    fixture.close().await;
    assert!(
        violations.is_empty(),
        "page requests exceeded {budget_ms:.0} ms during queue removals: {}",
        violations.join(", ")
    );
}
