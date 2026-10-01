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
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_graphql::{Request, Variables};
use chrono::Utc;
use futures::future::join_all;
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
        fs::remove_dir_all(&self.dir).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
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
