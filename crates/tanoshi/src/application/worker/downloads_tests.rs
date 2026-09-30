use super::*;
use bytes::Bytes;
use sqlx::SqlitePool;
use std::{
    io::Read,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tanoshi_lib::prelude::{ChapterInfo, Extension, Input, Lang, MangaInfo, SourceInfo};
use tanoshi_vm::prelude::Source;

use crate::infrastructure::{
    database::establish_connection,
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

struct TestExtension(ImageHandler);

impl Extension for TestExtension {
    fn get_source_info(&self) -> SourceInfo {
        SourceInfo {
            id: 1,
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
        unreachable!()
    }
    fn get_image_bytes(&self, url: String) -> Result<Bytes> {
        self.0(url)
    }
}

struct Fixture {
    dir: PathBuf,
    pool: SqlitePool,
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
                .execute(&*pool).await.unwrap();
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
            pool: (*pool).clone(),
            repo,
            calls: Arc::default(),
        }
    }

    async fn worker(&self, handler: Option<ImageHandler>) -> (Worker, DownloadSender) {
        let ext = ExtensionManager::new(self.dir.join("plugins"));
        let calls = self.calls.clone();
        ext.insert(Source::from(Box::new(TestExtension(Arc::new(
            move |url| {
                calls.lock().unwrap().push(url.clone());
                if let Some(handler) = &handler {
                    handler(url)
                } else {
                    Ok(Bytes::from(url))
                }
            },
        )))))
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
        let path = self.archive_path(chapter, temporary);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut zip = ZipWriter::new(File::create(path).unwrap());
        for rank in 0..pages {
            zip.start_file(
                format!("{rank:04}_{rank}.jpg"),
                SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(format!("https://example.test/{chapter}/{rank}.jpg").as_bytes())
                .unwrap();
        }
        zip.finish().unwrap();
    }

    async fn mark_pages(&self, chapter: i64, pages: i64) {
        sqlx::query(
            "UPDATE download_queue SET downloaded = true WHERE chapter_id = ? AND rank < ?",
        )
        .bind(chapter)
        .bind(pages)
        .execute(&self.pool)
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
async fn restart_resumes_valid_partial_archive_during_image_request() {
    let fixture = Fixture::new().await;
    fixture.write_archive(1, 1, true);
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
