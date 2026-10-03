use super::*;
use bytes::Bytes;
use std::{
    cell::Cell,
    future::Future,
    io::Read,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tanoshi_lib::prelude::{ChapterInfo, Extension, Input, Lang, MangaInfo, SourceInfo};
use tanoshi_vm::prelude::Source;

use crate::domain::services::download::DownloadService;
use crate::infrastructure::{
    database::{Pool, establish_connection},
    domain::repositories::{
        chapter::ChapterRepositoryImpl, download::DownloadRepositoryImpl,
        library::LibraryRepositoryImpl, manga::MangaRepositoryImpl,
    },
    notification,
};

type Worker = DownloadWorker<
    ChapterRepositoryImpl,
    DownloadRepositoryImpl,
    MangaRepositoryImpl,
    LibraryRepositoryImpl,
>;

type ImageHandler = Arc<dyn Fn(String) -> Result<Bytes> + Send + Sync>;

struct PausedWrite {
    path: PathBuf,
    started: tokio::sync::mpsc::UnboundedSender<()>,
    release: std::sync::mpsc::Receiver<()>,
}

static PAUSED_WRITES: Mutex<Vec<PausedWrite>> = Mutex::new(Vec::new());

pub(super) fn before_archive_write(path: &Path) {
    let pause = {
        let mut writes = PAUSED_WRITES.lock().unwrap();
        writes
            .iter()
            .position(|write| write.path == path)
            .map(|index| writes.swap_remove(index))
    };
    if let Some(pause) = pause {
        let _ = pause.started.send(());
        let _ = pause.release.recv();
    }
}

struct TestExtension(ImageHandler, Vec<String>, i64);

impl Extension for TestExtension {
    fn get_source_info(&self) -> SourceInfo {
        SourceInfo {
            id: self.2,
            name: "Test source".into(),
            url: "https://example.test".into(),
            version: "test",
            icon: "",
            languages: Lang::All,
            nsfw: false,
        }
    }

    fn get_popular_manga(&self, _: i64) -> Result<Vec<MangaInfo>> {
        unreachable!()
    }
    fn get_latest_manga(&self, _: i64) -> Result<Vec<MangaInfo>> {
        unreachable!()
    }
    fn search_manga(
        &self,
        _: i64,
        _: Option<String>,
        _: Option<Vec<Input>>,
    ) -> Result<Vec<MangaInfo>> {
        unreachable!()
    }
    fn get_manga_detail(&self, _: String) -> Result<MangaInfo> {
        unreachable!()
    }
    fn get_chapters(&self, _: String) -> Result<Vec<ChapterInfo>> {
        unreachable!()
    }
    fn get_pages(&self, _: String) -> Result<Vec<String>> {
        Ok(self.1.clone())
    }
    fn get_image_bytes(&self, url: String) -> Result<Bytes> {
        self.0(url)
    }
}

struct Fixture {
    dir: PathBuf,
    pool: Pool,
    repo: DownloadRepositoryImpl,
    calls: Arc<Mutex<Vec<String>>>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

impl Fixture {
    async fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-download-test-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = establish_connection(dir.join("test.db").to_str().unwrap(), true)
            .await
            .unwrap();
        for id in 1..=2 {
            sqlx::query("INSERT INTO chapter (id, source_id, manga_id, title, path, number, uploaded, date_added) VALUES (?, 1, 1, 'Chapter', ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(format!("/chapter/{id}")).bind(id)
                .execute(pool.write()).await.unwrap();
        }
        let repo = DownloadRepositoryImpl::new(pool.clone());
        let date_added = Utc::now().naive_utc();
        let mut queue = vec![];
        for (chapter_id, pages) in [(1, 2), (2, 1)] {
            for rank in 0..pages {
                queue.push(DownloadQueue {
                    id: 0,
                    source_id: 1,
                    source_name: "Test source".into(),
                    manga_id: 1,
                    manga_title: "Manga".into(),
                    chapter_id,
                    chapter_title: format!("Chapter {chapter_id}"),
                    rank,
                    url: format!("https://example.test/{chapter_id}/{rank}.jpg"),
                    priority: chapter_id + 10,
                    date_added,
                });
            }
        }
        repo.insert_download_queue(&queue).await.unwrap();
        Self {
            dir,
            pool,
            repo,
            calls: Arc::default(),
        }
    }

    async fn worker(&self, handler: Option<ImageHandler>) -> (Worker, DownloadSender) {
        self.worker_with_pages(handler, vec![]).await
    }

    async fn worker_with_pages(
        &self,
        handler: Option<ImageHandler>,
        pages: Vec<String>,
    ) -> (Worker, DownloadSender) {
        self.worker_with_options(handler, pages, Default::default())
            .await
    }

    async fn worker_with_options(
        &self,
        handler: Option<ImageHandler>,
        pages: Vec<String>,
        options: tanoshi_vm::extension::ExtensionManagerOptions,
    ) -> (Worker, DownloadSender) {
        let ext = ExtensionManager::new_with_options(self.dir.join("plugins"), options);
        let calls = self.calls.clone();
        ext.insert(Source::from(Box::new(TestExtension(
            Arc::new(move |url| {
                calls.lock().unwrap().push(url.clone());
                if let Some(handler) = &handler {
                    handler(url)
                } else {
                    Ok(Bytes::from(url))
                }
            }),
            pages,
            1,
        ))))
        .await
        .unwrap();
        let (tx, rx) = channel();
        let (_, updates) = tokio::sync::broadcast::channel(1);
        let worker = DownloadWorker::new(
            &self.dir,
            ChapterRepositoryImpl::new(self.pool.clone()),
            MangaRepositoryImpl::new(self.pool.clone()),
            self.repo.clone(),
            LibraryRepositoryImpl::new(self.pool.clone()),
            ext,
            notification::Builder::new(UserRepositoryImpl::new(self.pool.clone())).finish(),
            rx,
            updates,
            false,
        );
        (worker, tx)
    }

    fn archive_path(&self, chapter: i64, temporary: bool) -> PathBuf {
        self.dir.join("Test source").join("Manga").join(format!(
            "Chapter {chapter}{}.cbz",
            if temporary { ".temp" } else { "" }
        ))
    }

    fn write_archive(&self, chapter: i64, pages: usize, temporary: bool) {
        self.write_archive_with_options(chapter, pages, temporary, SimpleFileOptions::default());
    }

    fn write_archive_with_options(
        &self,
        chapter: i64,
        pages: usize,
        temporary: bool,
        options: SimpleFileOptions,
    ) {
        let path = self.archive_path(chapter, temporary);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut zip = ZipWriter::new(File::create(path).unwrap());
        for rank in 0..pages {
            zip.start_file(format!("{rank:04}_{rank}.jpg"), options)
                .unwrap();
            zip.write_all(format!("https://example.test/{chapter}/{rank}.jpg").as_bytes())
                .unwrap();
        }
        zip.finish().unwrap();
    }

    fn corrupt_archive_page(&self, chapter: i64, temporary: bool) {
        let path = self.archive_path(chapter, temporary);
        let mut zip = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let start = zip.by_index(0).unwrap().data_start().unwrap() as usize;
        drop(zip);
        let mut data = fs::read(&path).unwrap();
        data[start] ^= 0xff;
        fs::write(&path, data).unwrap();

        // The directory and entry names survive, but reading the page fails.
        let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
        assert!(
            zip.by_index(0)
                .unwrap()
                .read_to_end(&mut Vec::new())
                .is_err()
        );
    }

    async fn mark_pages(&self, chapter: i64, pages: i64) {
        sqlx::query(
            "UPDATE download_queue SET downloaded = true WHERE chapter_id = ? AND rank < ?",
        )
        .bind(chapter)
        .bind(pages)
        .execute(self.pool.write())
        .await
        .unwrap();
    }

    async fn assert_finished(&self) {
        assert!(self.repo.get_download_queue(&[]).await.unwrap().is_empty());
        for (chapter, pages) in [(1, 2), (2, 1)] {
            let path = self.archive_path(chapter, false);
            assert_eq!(
                self.repo
                    .get_chapter_downloaded_path(chapter)
                    .await
                    .unwrap(),
                path.to_str().unwrap()
            );
            let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
            assert_eq!(zip.len(), pages);
            for rank in 0..pages {
                let mut data = String::new();
                zip.by_name(&format!("{rank:04}_{rank}.jpg"))
                    .unwrap()
                    .read_to_string(&mut data)
                    .unwrap();
                assert_eq!(data, format!("https://example.test/{chapter}/{rank}.jpg"));
            }
        }
    }

    async fn drain(&self, worker: &mut Worker) {
        for _ in 0..8 {
            if !worker.download().await.unwrap() {
                self.assert_finished().await;
                return;
            }
        }
        panic!("queue failed to drain");
    }
}

#[tokio::test]
async fn download_worker_yields_the_next_source_slot_to_browsing() {
    use std::task::{Context, Waker};
    use tanoshi_vm::extension::ExtensionManagerOptions;

    let fixture = Fixture::new().await;
    let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
    let (release, receiver) = std::sync::mpsc::channel();
    let receiver = Mutex::new(receiver);
    let handler: ImageHandler = Arc::new(move |url| {
        if url == "hold" {
            let _ = started_tx.send(());
            // A dropped sender unblocks the native call if the test fails.
            let _ = receiver.lock().unwrap().recv();
        }
        Ok(Bytes::from(url))
    });
    let (worker, _commands) = fixture
        .worker_with_options(
            Some(handler),
            vec![],
            ExtensionManagerOptions {
                max_concurrent_calls: 1,
                ..Default::default()
            },
        )
        .await;
    let browsing = worker.ext.clone().with_priority(RequestPriority::High);
    let active = tokio::spawn({
        let browsing = browsing.clone();
        async move { browsing.get_image_bytes(1, "hold".into()).await }
    });
    started_rx.recv().await.unwrap();
    let mut download = Box::pin(worker.ext.get_image_bytes(1, "download.jpg".into()));
    let mut thumbnail = Box::pin(browsing.get_image_bytes(1, "thumbnail.jpg".into()));
    assert!(
        download
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert!(
        thumbnail
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    release.send(()).unwrap();
    active.await.unwrap().unwrap();
    let (download_result, thumbnail_result) = tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(download, thumbnail)
    })
    .await
    .unwrap();
    download_result.unwrap();
    thumbnail_result.unwrap();
    assert_eq!(
        *fixture.calls.lock().unwrap(),
        ["hold", "thumbnail.jpg", "download.jpg"]
    );
}

// Run on a current-thread runtime so synchronous archive work would starve the timer.
async fn assert_timers_run_during<T>(work: impl Future<Output = T>) -> T {
    let finished = Cell::new(false);
    let (timer_ran_during_work, result) = tokio::join!(
        biased;
        async {
            sleep(Duration::from_millis(1)).await;
            !finished.get()
        },
        async {
            let result = work.await;
            finished.set(true);
            result
        }
    );
    assert!(
        timer_ran_during_work,
        "archive work starved the Tokio timer"
    );
    result
}

#[tokio::test(flavor = "current_thread")]
async fn archive_writes_keep_timers_responsive() {
    let fixture = Fixture::new().await;
    let tmp = fixture.archive_path(1, true);
    let data = Bytes::from(vec![0x5a; 32 * 1024 * 1024]);
    for (filename, append) in [("0000_0.jpg", false), ("0001_1.jpg", true)] {
        assert_timers_run_during(Worker::write_page(
            tmp.parent().unwrap(),
            &tmp,
            filename,
            data.clone(),
            append,
        ))
        .await
        .unwrap();
    }
    let mut zip = ZipArchive::new(File::open(&tmp).unwrap()).unwrap();
    assert_eq!(zip.len(), 2);
    for index in 0..2 {
        let mut entry = zip.by_index(index).unwrap();
        assert_eq!(entry.compression(), zip::CompressionMethod::Stored);
        assert_eq!(entry.compressed_size(), entry.size());
        let mut page = Vec::new();
        entry.read_to_end(&mut page).unwrap();
        assert_eq!(page, data.as_ref());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn archive_validation_keeps_timers_responsive() {
    let fixture = Fixture::new().await;
    let tmp = fixture.archive_path(1, true);
    Worker::write_page(
        tmp.parent().unwrap(),
        &tmp,
        "0000_0.jpg",
        Bytes::from(vec![0x5a; 32 * 1024 * 1024]),
        false,
    )
    .await
    .unwrap();
    let archive = assert_timers_run_during(Worker::open_archive(&tmp, true))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(archive.len(), 1);
}

#[tokio::test]
async fn restart_recovers_corrupt_archive_and_preserves_queue_order() {
    let fixture = Fixture::new().await;
    // An older completed archive must not mask a corrupt re-download.
    fixture.write_archive(1, 2, false);
    fixture.mark_pages(1, 1).await;
    let tmp = fixture.archive_path(1, true);
    fs::create_dir_all(tmp.parent().unwrap()).unwrap();
    fs::write(&tmp, b"interrupted ZIP without a central directory").unwrap();
    let (mut worker, _) = fixture.worker(None).await;

    assert!(worker.download().await.unwrap());
    let queue = fixture.repo.get_download_queue(&[]).await.unwrap();
    assert_eq!(
        (queue[0].chapter_id, queue[0].downloaded, queue[0].priority),
        (1, 0, 11)
    );
    assert_eq!(queue[1].chapter_id, 2);
    assert!(!tmp.exists());
    fixture.drain(&mut worker).await;
    assert_eq!(fixture.calls.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn restart_recovers_missing_or_incomplete_archive() {
    for (completed_pages, archived_pages) in [(1, None), (2, None), (2, Some(1))] {
        let fixture = Fixture::new().await;
        fixture.mark_pages(1, completed_pages).await;
        if let Some(pages) = archived_pages {
            fixture.write_archive(1, pages, true);
        }
        let (mut worker, _) = fixture.worker(None).await;
        fixture.drain(&mut worker).await;
        assert_eq!(fixture.calls.lock().unwrap().len(), 3);
    }
}

#[tokio::test]
async fn restart_recovers_damaged_pages_with_intact_archive_directory() {
    for compression in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        for (completed_pages, archived_pages, temporary) in
            [(0, 1, true), (1, 1, true), (2, 2, true), (2, 2, false)]
        {
            let fixture = Fixture::new().await;
            fixture.write_archive_with_options(
                1,
                archived_pages,
                temporary,
                SimpleFileOptions::default().compression_method(compression),
            );
            fixture.corrupt_archive_page(1, temporary);
            fixture.mark_pages(1, completed_pages).await;
            let (mut worker, _) = fixture.worker(None).await;

            assert!(worker.download().await.unwrap());
            let queue = fixture.repo.get_download_queue(&[]).await.unwrap();
            assert_eq!(
                (queue[0].chapter_id, queue[0].downloaded, queue[0].priority),
                (1, 0, 11)
            );
            assert!(!fixture.archive_path(1, true).exists());
            assert!(fixture.calls.lock().unwrap().is_empty());
            fixture.drain(&mut worker).await;
            assert_eq!(fixture.calls.lock().unwrap().len(), 3);
        }
    }
}

#[tokio::test]
async fn retry_revalidates_archive_after_failed_database_update() {
    let fixture = Fixture::new().await;
    let (mut worker, _) = fixture.worker(None).await;
    assert!(worker.download().await.unwrap());

    // Fail after appending the next page, leaving the queue partially complete.
    sqlx::query("CREATE TRIGGER fail_download_update BEFORE UPDATE OF downloaded ON download_queue WHEN NEW.chapter_id = 1 AND NEW.rank = 1 BEGIN SELECT RAISE(ABORT, 'temporary database failure'); END")
        .execute(fixture.pool.write()).await.unwrap();
    assert!(worker.download().await.is_err());
    sqlx::query("DROP TRIGGER fail_download_update")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    fixture.corrupt_archive_page(1, true);

    // The retry must inspect the contents again before trusting either page.
    assert!(worker.download().await.unwrap());
    let queue = fixture.repo.get_download_queue(&[]).await.unwrap();
    assert_eq!(queue[0].downloaded, 0);
    assert!(!fixture.archive_path(1, true).exists());
    fixture.drain(&mut worker).await;
    assert_eq!(fixture.calls.lock().unwrap().len(), 5);
}

#[tokio::test]
async fn restart_resumes_valid_partial_archive_during_image_request() {
    let fixture = Fixture::new().await;
    fixture.write_archive_with_options(
        1,
        1,
        true,
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
    );
    fixture.mark_pages(1, 1).await;
    let tmp = fixture.archive_path(1, true);
    let (mut worker, _) = fixture
        .worker(Some(Arc::new(move |url| {
            if url.ends_with("/1/1.jpg") {
                // The existing pages must remain readable during the request.
                let mut zip = ZipArchive::new(File::open(&tmp)?)?;
                let mut data = String::new();
                zip.by_name("0000_0.jpg")?.read_to_string(&mut data)?;
                assert_eq!(data, "https://example.test/1/0.jpg");
            }
            Ok(Bytes::from(url))
        })))
        .await;
    fixture.drain(&mut worker).await;
    assert_eq!(fixture.calls.lock().unwrap().len(), 2);
    let mut zip = ZipArchive::new(File::open(fixture.archive_path(1, false)).unwrap()).unwrap();
    assert_eq!(
        zip.by_index(0).unwrap().compression(),
        zip::CompressionMethod::Deflated
    );
    assert_eq!(
        zip.by_index(1).unwrap().compression(),
        zip::CompressionMethod::Stored
    );
}

#[tokio::test]
async fn restart_keeps_page_written_before_database_update_without_duplicate() {
    let fixture = Fixture::new().await;
    fixture.write_archive(1, 1, true);
    let (mut worker, _) = fixture.worker(None).await;
    fixture.drain(&mut worker).await;
    assert_eq!(fixture.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn restart_finalizes_completed_chapters_before_and_after_rename() {
    for temporary in [true, false] {
        let fixture = Fixture::new().await;
        fixture.write_archive(1, 2, temporary);
        fixture.mark_pages(1, 2).await;
        let (mut worker, _) = fixture.worker(None).await;
        fixture.drain(&mut worker).await;
        assert_eq!(fixture.calls.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn cancellation_removes_all_chapters_when_worker_channel_is_closed() {
    let fixture = Fixture::new().await;
    let (worker, tx) = fixture.worker(None).await;
    drop(worker);
    let service = DownloadService::new(fixture.repo.clone(), tx);

    service
        .remove_chapters_from_queue(vec![1, 2])
        .await
        .unwrap();
    assert!(
        fixture
            .repo
            .get_download_queue(&[])
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn cancellation_cleans_partial_archives_while_paused() {
    let fixture = Fixture::new().await;
    fixture.write_archive(1, 1, true);
    fixture.mark_pages(1, 1).await;
    // Cancellation must preserve any previous completed archive.
    fixture.write_archive(1, 2, false);
    let final_path = fixture.archive_path(1, false);
    let final_contents = fs::read(&final_path).unwrap();
    let tmp = fixture.archive_path(1, true);
    fs::write(fixture.dir.join(".pause"), b"").unwrap();
    let (worker, tx) = fixture.worker(None).await;
    let service = DownloadService::new(fixture.repo.clone(), tx);
    let handle = tokio::spawn(worker.run());

    // Include a chapter without an archive, a repeated id, and an unknown id.
    service
        .remove_chapters_from_queue(vec![1, 2, 1, 999])
        .await
        .unwrap();
    assert!(
        fixture
            .repo
            .get_download_queue(&[])
            .await
            .unwrap()
            .is_empty()
    );
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        while tmp.exists() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    handle.abort();
    let _ = handle.await;
    result.unwrap();
    assert_eq!(fs::read(final_path).unwrap(), final_contents);
    assert!(fixture.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancelling_an_image_waiting_for_a_source_slot_unblocks_cleanup_and_other_sources() {
    use tanoshi_vm::extension::ExtensionManagerOptions;

    let fixture = Fixture::new().await;
    fixture.write_archive(1, 1, true);
    fixture.mark_pages(1, 1).await;
    let tmp = fixture.archive_path(1, true);
    let (started, mut starts) = tokio::sync::mpsc::unbounded_channel();
    let (release, receiver) = std::sync::mpsc::channel();
    let receiver = Mutex::new(receiver);
    let (worker, tx) = fixture
        .worker_with_options(
            Some(Arc::new(move |url| {
                if url == "hold" {
                    let _ = started.send(());
                    // A dropped sender releases the slot if an assertion fails.
                    let _ = receiver.lock().unwrap().recv();
                }
                Ok(Bytes::from(url))
            })),
            vec![],
            ExtensionManagerOptions {
                max_concurrent_calls: 1,
                ..Default::default()
            },
        )
        .await;
    let calls = fixture.calls.clone();
    worker
        .ext
        .insert(Source::from(Box::new(TestExtension(
            Arc::new(move |url| {
                calls.lock().unwrap().push(url.clone());
                Ok(Bytes::from(url))
            }),
            vec![],
            2,
        ))))
        .await
        .unwrap();
    sqlx::query("UPDATE download_queue SET source_id = 2 WHERE chapter_id = 2")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    let browsing = worker.ext.clone().with_priority(RequestPriority::High);
    let occupied_slot = tokio::spawn({
        let browsing = browsing.clone();
        async move { browsing.get_image_bytes(1, "hold".into()).await }
    });
    starts.recv().await.unwrap();
    let handle = tokio::spawn(worker.run());
    // Let the worker reach admission while the source's only slot is held.
    sleep(Duration::from_millis(100)).await;
    tx.send(Command::Download).unwrap();
    DownloadService::new(fixture.repo.clone(), tx)
        .remove_chapters_from_queue(vec![1])
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), async {
        while tmp.exists()
            || !fixture
                .repo
                .get_download_queue(&[])
                .await
                .unwrap()
                .is_empty()
        {
            sleep(Duration::from_millis(10)).await;
        }
        assert!(
            !occupied_slot.is_finished(),
            "test released the occupied slot too early"
        );
        assert_eq!(
            fixture.repo.get_chapter_downloaded_path(2).await.unwrap(),
            fixture.archive_path(2, false).to_str().unwrap()
        );
    })
    .await;
    release.send(()).unwrap();
    occupied_slot.await.unwrap().unwrap();
    handle.abort();
    let _ = handle.await;
    result.expect("cancellation waited for the source slot instead of unblocking the worker");
    assert_eq!(
        *fixture.calls.lock().unwrap(),
        ["hold", "https://example.test/2/0.jpg"]
    );
    assert!(!tmp.exists());
    assert!(!fixture.archive_path(1, false).exists());
}

#[tokio::test]
async fn cancellation_waits_for_archive_write_completion_before_cleanup() {
    let fixture = Fixture::new().await;
    fixture.write_archive(1, 1, true);
    fixture.mark_pages(1, 1).await;
    let tmp = fixture.archive_path(1, true);
    let (started, mut starts) = tokio::sync::mpsc::unbounded_channel();
    let (release, receiver) = std::sync::mpsc::channel();
    PAUSED_WRITES.lock().unwrap().push(PausedWrite {
        path: tmp.clone(),
        started,
        release: receiver,
    });
    let (worker, tx) = fixture.worker(None).await;
    let handle = tokio::spawn(worker.run());
    tokio::time::timeout(Duration::from_secs(2), starts.recv())
        .await
        .unwrap()
        .unwrap();
    // The blocking write has started, so cancellation must wait for it before
    // deleting the archive or allowing another page to be written.
    DownloadService::new(fixture.repo.clone(), tx)
        .remove_chapters_from_queue(vec![1])
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    assert!(
        tmp.exists(),
        "cleanup ran before the blocking write completed"
    );
    assert!(
        fixture
            .repo
            .get_chapter_downloaded_path(2)
            .await
            .unwrap()
            .is_empty()
    );
    release.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), async {
        while tmp.exists()
            || !fixture
                .repo
                .get_download_queue(&[])
                .await
                .unwrap()
                .is_empty()
        {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    handle.abort();
    let _ = handle.await;
    result.unwrap();
    assert!(!tmp.exists());
    assert!(!fixture.archive_path(1, false).exists());
    assert_eq!(
        fixture.repo.get_chapter_downloaded_path(2).await.unwrap(),
        fixture.archive_path(2, false).to_str().unwrap()
    );
}

#[tokio::test]
async fn cancellation_during_image_request_cleans_archive_and_continues_queue() {
    for completed_pages in [0, 1] {
        let fixture = Fixture::new().await;
        if completed_pages > 0 {
            fixture.write_archive(1, completed_pages, true);
            fixture.mark_pages(1, completed_pages as i64).await;
        }
        let tmp = fixture.archive_path(1, true);
        let sender = Arc::new(Mutex::new(None::<DownloadSender>));
        let request_sender = sender.clone();
        let repo = fixture.repo.clone();
        let runtime = tokio::runtime::Handle::current();
        let (worker, tx) = fixture
            .worker(Some(Arc::new(move |url| {
                if url.ends_with(&format!("/1/{completed_pages}.jpg")) {
                    let tx = request_sender.lock().unwrap().as_ref().unwrap().clone();
                    let service = DownloadService::new(repo.clone(), tx);
                    runtime.block_on(service.remove_chapters_from_queue(vec![1]))?;
                }
                Ok(Bytes::from(url))
            })))
            .await;
        *sender.lock().unwrap() = Some(tx);
        let handle = tokio::spawn(worker.run());
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            while tmp.exists()
                || !fixture
                    .repo
                    .get_download_queue(&[])
                    .await
                    .unwrap()
                    .is_empty()
            {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        handle.abort();
        let _ = handle.await;
        result.unwrap();

        assert!(!fixture.archive_path(1, false).exists());
        let cancelled_path: Option<String> =
            sqlx::query_scalar("SELECT downloaded_path FROM chapter WHERE id = 1")
                .fetch_one(fixture.pool.read())
                .await
                .unwrap();
        assert_eq!(cancelled_path, None);
        assert_eq!(
            fixture.repo.get_chapter_downloaded_path(2).await.unwrap(),
            fixture.archive_path(2, false).to_str().unwrap()
        );
        assert_eq!(fixture.calls.lock().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn stale_cleanup_keeps_the_requeued_chapters_image_request() {
    let fixture = Fixture::new().await;
    let cancelled = fixture
        .repo
        .get_download_queue(&[1])
        .await
        .unwrap()
        .remove(0);
    let mut first = fixture
        .repo
        .get_single_download_queue()
        .await
        .unwrap()
        .unwrap();
    fixture
        .repo
        .delete_download_queue_by_chapter_id(1)
        .await
        .unwrap();
    first.url = "https://example.test/1/requeued.jpg".into();
    let mut second = first.clone();
    second.rank = 1;
    second.url = "https://example.test/1/next.jpg".into();
    fixture
        .repo
        .insert_download_queue(&[first, second])
        .await
        .unwrap();

    let (started, mut starts) = tokio::sync::mpsc::unbounded_channel();
    let (release, receiver) = std::sync::mpsc::channel();
    let receiver = Mutex::new(receiver);
    let (mut worker, tx) = fixture
        .worker(Some(Arc::new(move |url| {
            if url.ends_with("/requeued.jpg") {
                let _ = started.send(());
                // Dropping the sender unblocks the request if the test fails.
                let _ = receiver.lock().unwrap().recv();
            }
            Ok(Bytes::from(url))
        })))
        .await;
    let fetch = tokio::spawn(async move {
        let result = worker.download().await;
        (worker, result)
    });
    starts.recv().await.unwrap();
    tx.send(Command::CleanupCancelledChapter(cancelled))
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    let stopped_early = fetch.is_finished();
    release.send(()).unwrap();
    let (_worker, result) = fetch.await.unwrap();
    assert!(result.unwrap());
    assert!(
        !stopped_early,
        "stale cleanup discarded the active request for the requeued chapter"
    );
    assert_eq!(
        *fixture.calls.lock().unwrap(),
        ["https://example.test/1/requeued.jpg"]
    );
    let mut zip = ZipArchive::new(File::open(fixture.archive_path(1, true)).unwrap()).unwrap();
    let mut data = String::new();
    zip.by_name("0000_requeued.jpg")
        .unwrap()
        .read_to_string(&mut data)
        .unwrap();
    assert_eq!(data, "https://example.test/1/requeued.jpg");
    assert_eq!(
        fixture.repo.get_download_queue(&[1]).await.unwrap()[0].downloaded,
        1
    );
}

#[tokio::test]
async fn cancellation_cleanup_preserves_requeued_chapter() {
    let fixture = Fixture::new().await;
    let cancelled = fixture
        .repo
        .get_download_queue(&[1])
        .await
        .unwrap()
        .remove(0);
    let first = fixture
        .repo
        .get_single_download_queue()
        .await
        .unwrap()
        .unwrap();
    let mut second = first.clone();
    second.rank = 1;
    second.url = "https://example.test/1/1.jpg".into();
    fixture
        .repo
        .delete_download_queue_by_chapter_id(1)
        .await
        .unwrap();
    fixture
        .repo
        .insert_download_queue(&[first, second])
        .await
        .unwrap();
    fixture.write_archive(1, 1, true);
    fixture.mark_pages(1, 1).await;
    let tmp = fixture.archive_path(1, true);
    let contents = fs::read(&tmp).unwrap();
    let (mut worker, _) = fixture.worker(None).await;

    worker.cleanup_cancelled_chapter(&cancelled).await.unwrap();
    assert_eq!(fs::read(tmp).unwrap(), contents);
    fixture.drain(&mut worker).await;
    assert_eq!(fixture.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn requeue_before_cleanup_discards_pages_with_old_urls() {
    let fixture = Fixture::new().await;
    sqlx::query("INSERT INTO manga (id, source_id, title, author, genre, path, cover_url, date_added) VALUES (1, 1, 'Manga', '[]', '[]', '/manga/1', '', CURRENT_TIMESTAMP)")
        .execute(fixture.pool.write()).await.unwrap();
    // Match the archive name produced by the real insertion path.
    sqlx::query("UPDATE download_queue SET chapter_title = '1 - Chapter' WHERE chapter_id = 1")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    let pages = vec![
        "https://example.test/1/new-0.jpg".to_string(),
        "https://example.test/1/new-1.jpg".to_string(),
    ];
    let (mut worker, tx) = fixture.worker_with_pages(None, pages.clone()).await;
    assert!(worker.download().await.unwrap());
    let manga_path = fixture.dir.join("Test source").join("Manga");
    let tmp = manga_path.join("1 - Chapter.temp.cbz");
    let archive_path = manga_path.join("1 - Chapter.cbz");
    assert!(tmp.exists());
    fs::write(fixture.dir.join(".pause"), b"").unwrap();

    // Delay the cleanup behind a requeue command while the old archive exists.
    tx.send(Command::InsertIntoQueue(1)).unwrap();
    let service = DownloadService::new(fixture.repo.clone(), tx);
    service.remove_chapters_from_queue(vec![1]).await.unwrap();
    let handle = tokio::spawn(worker.run());
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let queue = fixture.repo.get_download_queue(&[1]).await.unwrap();
            if !queue.is_empty() && !tmp.exists() {
                assert_eq!((queue[0].downloaded, queue[0].total), (0, 2));
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
        service
            .change_download_status(&fixture.dir, true)
            .await
            .unwrap();
        while !fixture
            .repo
            .get_download_queue(&[])
            .await
            .unwrap()
            .is_empty()
        {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    handle.abort();
    let _ = handle.await;
    result.unwrap();

    let mut zip = ZipArchive::new(File::open(&archive_path).unwrap()).unwrap();
    assert_eq!(zip.len(), pages.len());
    for (rank, page) in pages.iter().enumerate() {
        let mut data = String::new();
        zip.by_name(&format!("{rank:04}_new-{rank}.jpg"))
            .unwrap()
            .read_to_string(&mut data)
            .unwrap();
        assert_eq!(&data, page);
    }
    assert_eq!(fixture.calls.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn startup_respects_pause_and_resume_drains_saved_queue() {
    let fixture = Fixture::new().await;
    fs::write(fixture.dir.join(".pause"), b"").unwrap();
    let (worker, tx) = fixture.worker(None).await;
    let handle = tokio::spawn(worker.run());
    sleep(Duration::from_millis(100)).await;
    assert!(fixture.calls.lock().unwrap().is_empty());
    fs::remove_file(fixture.dir.join(".pause")).unwrap();
    tx.send(Command::Download).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        while !fixture
            .repo
            .get_download_queue(&[])
            .await
            .unwrap()
            .is_empty()
        {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    handle.abort();
    let _ = handle.await;
    result.unwrap();
    fixture.assert_finished().await;
}

#[tokio::test]
async fn worker_retries_after_failure_without_another_command() {
    let fixture = Fixture::new().await;
    let failures = Arc::new(AtomicUsize::new(0));
    let (worker, _tx) = fixture
        .worker(Some(Arc::new(move |url| {
            if failures.fetch_add(1, Ordering::Relaxed) < MAX_RETRIES {
                anyhow::bail!("temporary network failure");
            }
            Ok(Bytes::from(url))
        })))
        .await;
    let handle = tokio::spawn(worker.run());
    let result = tokio::time::timeout(QUEUE_RETRY_DELAY + Duration::from_secs(15), async {
        while !fixture
            .repo
            .get_download_queue(&[])
            .await
            .unwrap()
            .is_empty()
        {
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    handle.abort();
    let _ = handle.await;
    result.unwrap();
    fixture.assert_finished().await;
    assert_eq!(fixture.calls.lock().unwrap().len(), 6);
}
