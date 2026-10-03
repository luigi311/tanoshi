use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, atomic::Ordering, mpsc as blocking_mpsc},
    task::{Context, Poll, Waker},
    time::Duration,
};

use anyhow::{Result, bail};
use bytes::Bytes;
use tanoshi_lib::prelude::{ChapterInfo, Extension, Input, MangaInfo, SourceInfo};
use tokio::sync::mpsc;

use super::{
    ExtensionManager, ExtensionManagerOptions, RequestPriority, Source, SourceAdmission,
    SourceEntry, UNIQUE_PATH_COUNTER, dummy_source_info,
};

struct ImageExtension {
    source_id: i64,
    started: mpsc::UnboundedSender<String>,
    release: Mutex<blocking_mpsc::Receiver<()>>,
}

impl Extension for ImageExtension {
    fn get_source_info(&self) -> SourceInfo {
        dummy_source_info(self.source_id)
    }

    fn get_image_bytes(&self, url: String) -> Result<Bytes> {
        let _ = self.started.send(url.clone());
        if url.starts_with("hold") {
            // Dropping the test's sender also unblocks calls during test failure.
            let _ = self.release.lock().unwrap().recv();
        }
        Ok(Bytes::from(url))
    }

    fn get_popular_manga(&self, _: i64) -> Result<Vec<MangaInfo>> {
        bail!("unused test operation")
    }
    fn get_latest_manga(&self, _: i64) -> Result<Vec<MangaInfo>> {
        bail!("unused test operation")
    }
    fn search_manga(
        &self,
        _: i64,
        _: Option<String>,
        _: Option<Vec<Input>>,
    ) -> Result<Vec<MangaInfo>> {
        bail!("unused test operation")
    }
    fn get_manga_detail(&self, _: String) -> Result<MangaInfo> {
        bail!("unused test operation")
    }
    fn get_chapters(&self, _: String) -> Result<Vec<ChapterInfo>> {
        bail!("unused test operation")
    }
    fn get_pages(&self, _: String) -> Result<Vec<String>> {
        bail!("unused test operation")
    }
}

fn manager(options: ExtensionManagerOptions) -> ExtensionManager {
    let id = UNIQUE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    ExtensionManager::new_with_options(
        std::env::temp_dir().join(format!("tanoshi-vm-queue-{}-{id}", std::process::id())),
        options,
    )
}

fn insert_source(
    manager: &ExtensionManager,
    source_id: i64,
) -> (
    Arc<SourceEntry>,
    mpsc::UnboundedReceiver<String>,
    blocking_mpsc::Sender<()>,
) {
    let (started, requests) = mpsc::unbounded_channel();
    let (release, receiver) = blocking_mpsc::channel();
    let entry = Arc::new(
        Source::from(Box::new(ImageExtension {
            source_id,
            started,
            release: Mutex::new(receiver),
        }))
        .into_entry(manager.options.max_concurrent_calls)
        .unwrap(),
    );
    manager.insert_entry(entry.clone()).unwrap();
    (entry, requests, release)
}

fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn browsing_runs_before_queued_background_images() {
    let manager = manager(ExtensionManagerOptions {
        max_concurrent_calls: 1,
        ..Default::default()
    });
    let (_, mut requests, release) = insert_source(&manager, 1);
    let background = manager.clone().with_priority(RequestPriority::Low);
    let active = tokio::spawn({
        let background = background.clone();
        async move { background.get_image_bytes(1, "hold".into()).await }
    });
    assert_eq!(requests.recv().await.unwrap(), "hold");
    let mut low_one = Box::pin(background.get_image_bytes(1, "background-1".into()));
    let mut low_two = Box::pin(background.get_image_bytes(1, "background-2".into()));
    let mut high_one = Box::pin(manager.get_image_bytes(1, "user-1".into()));
    let mut high_two = Box::pin(manager.get_image_bytes(1, "user-2".into()));
    assert!(poll_once(low_one.as_mut()).is_pending());
    assert!(poll_once(low_two.as_mut()).is_pending());
    assert!(poll_once(high_one.as_mut()).is_pending());
    assert!(poll_once(high_two.as_mut()).is_pending());
    assert!(
        requests.try_recv().is_err(),
        "running background call was preempted"
    );
    release.send(()).unwrap();
    active.await.unwrap().unwrap();
    let (h1, h2, l1, l2) = tokio::join!(high_one, high_two, low_one, low_two);
    for result in [h1, h2, l1, l2] {
        result.unwrap();
    }
    for expected in ["user-1", "user-2", "background-1", "background-2"] {
        assert_eq!(requests.recv().await.unwrap(), expected);
    }
}

#[tokio::test]
async fn large_thumbnail_queue_survives_saturation_and_other_sources_keep_running() {
    let manager = manager(ExtensionManagerOptions::default());
    let (_, mut requests, release) = insert_source(&manager, 1);
    let (_, mut other_requests, _other_release) = insert_source(&manager, 2);
    let mut active = Vec::new();
    for index in 0..manager.options.max_concurrent_calls {
        active.push(tokio::spawn({
            let manager = manager.clone();
            async move { manager.get_image_bytes(1, format!("hold-{index}")).await }
        }));
    }
    for _ in 0..manager.options.max_concurrent_calls {
        assert!(requests.recv().await.unwrap().starts_with("hold"));
    }
    let mut thumbnails = Vec::new();
    for index in 0..400 {
        let mut request = Box::pin(manager.get_image_bytes(1, format!("thumbnail-{index}")));
        assert!(poll_once(request.as_mut()).is_pending());
        thumbnails.push(request);
    }
    tokio::time::timeout(
        Duration::from_secs(1),
        manager.get_image_bytes(2, "other-source".into()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(other_requests.recv().await.unwrap(), "other-source");
    // This exceeds the old one-second admission timeout while every slot is held.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    for thumbnail in &mut thumbnails {
        assert!(
            poll_once(thumbnail.as_mut()).is_pending(),
            "queued thumbnail was rejected before a slot became available"
        );
    }
    assert!(
        requests.try_recv().is_err(),
        "source exceeded its active-call limit"
    );
    for _ in 0..manager.options.max_concurrent_calls {
        release.send(()).unwrap();
    }
    for task in active {
        task.await.unwrap().unwrap();
    }
    for (index, thumbnail) in thumbnails.into_iter().enumerate() {
        assert_eq!(
            thumbnail.await.unwrap(),
            Bytes::from(format!("thumbnail-{index}"))
        );
    }
}

#[tokio::test]
async fn queue_wait_does_not_consume_the_image_execution_timeout() {
    let manager = manager(ExtensionManagerOptions {
        max_concurrent_calls: 1,
        image_timeout: Duration::from_millis(100),
        ..Default::default()
    });
    let (entry, _, _release) = insert_source(&manager, 1);
    let active = entry.queue.acquire(RequestPriority::Low).await.unwrap();
    let mut image = Box::pin(manager.get_image_bytes(1, "image".into()));
    assert!(poll_once(image.as_mut()).is_pending());
    tokio::time::sleep(Duration::from_millis(200)).await;
    drop(active);
    assert_eq!(image.await.unwrap(), Bytes::from_static(b"image"));
}

#[tokio::test]
async fn abandoned_native_call_wakes_queued_requests_when_all_slots_are_stuck() {
    let manager = manager(ExtensionManagerOptions {
        max_concurrent_calls: 1,
        image_timeout: Duration::from_millis(100),
        ..Default::default()
    });
    let (entry, mut requests, release) = insert_source(&manager, 1);
    let active = tokio::spawn({
        let manager = manager.clone();
        async move { manager.get_image_bytes(1, "hold".into()).await }
    });
    assert_eq!(requests.recv().await.unwrap(), "hold");
    let mut queued = Box::pin(manager.get_image_bytes(1, "queued".into()));
    assert!(poll_once(queued.as_mut()).is_pending());
    assert!(
        active
            .await
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("extension-timeout")
    );
    let error = tokio::time::timeout(Duration::from_secs(1), queued)
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("extension-circuit-open"));
    assert!(requests.try_recv().is_err());
    release.send(()).unwrap();
    let permit = tokio::time::timeout(
        Duration::from_secs(1),
        entry.queue.acquire(RequestPriority::High),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(entry.health.admission(), SourceAdmission::Allowed);
    drop(permit);
}

#[tokio::test]
async fn cancelling_a_native_caller_keeps_its_slot_and_execution_deadline() {
    let manager = manager(ExtensionManagerOptions {
        max_concurrent_calls: 1,
        image_timeout: Duration::from_millis(100),
        ..Default::default()
    });
    let (entry, mut requests, release) = insert_source(&manager, 1);
    let active = tokio::spawn({
        let manager = manager.clone();
        async move { manager.get_image_bytes(1, "hold".into()).await }
    });
    assert_eq!(requests.recv().await.unwrap(), "hold");
    let mut queued = Box::pin(manager.get_image_bytes(1, "queued".into()));
    assert!(poll_once(queued.as_mut()).is_pending());
    active.abort();
    assert!(active.await.unwrap_err().is_cancelled());
    let error = tokio::time::timeout(Duration::from_secs(1), queued)
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("extension-circuit-open"));
    assert!(
        requests.try_recv().is_err(),
        "cancellation released a running call's slot"
    );
    release.send(()).unwrap();
    drop(
        tokio::time::timeout(
            Duration::from_secs(1),
            entry.queue.acquire(RequestPriority::High),
        )
        .await
        .unwrap()
        .unwrap(),
    );
}

#[tokio::test]
async fn quarantine_wakes_waiters_without_dispatching_them() {
    let manager = manager(ExtensionManagerOptions {
        max_concurrent_calls: 1,
        ..Default::default()
    });
    let (entry, mut requests, _release) = insert_source(&manager, 1);
    let active = entry.queue.acquire(RequestPriority::Low).await.unwrap();
    let mut image = Box::pin(manager.get_image_bytes(1, "queued".into()));
    assert!(poll_once(image.as_mut()).is_pending());
    entry.health.quarantine();
    let error = tokio::time::timeout(Duration::from_secs(1), image)
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("extension-quarantined"));
    drop(active);
    assert!(requests.try_recv().is_err());
}

#[tokio::test]
async fn replacing_and_unloading_a_source_wakes_its_waiters() {
    let manager = manager(ExtensionManagerOptions {
        max_concurrent_calls: 1,
        ..Default::default()
    });
    let (old_entry, mut old_requests, _old_release) = insert_source(&manager, 1);
    let old_active = old_entry.queue.acquire(RequestPriority::Low).await.unwrap();
    let mut old_image = Box::pin(manager.get_image_bytes(1, "old-queued".into()));
    assert!(poll_once(old_image.as_mut()).is_pending());
    let (new_entry, mut new_requests, _new_release) = insert_source(&manager, 1);
    let error = tokio::time::timeout(Duration::from_secs(1), old_image)
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("extension-admission"));
    manager
        .get_image_bytes(1, "new-image".into())
        .await
        .unwrap();
    assert_eq!(new_requests.recv().await.unwrap(), "new-image");
    let new_active = new_entry.queue.acquire(RequestPriority::Low).await.unwrap();
    let mut new_image = Box::pin(manager.get_image_bytes(1, "new-queued".into()));
    assert!(poll_once(new_image.as_mut()).is_pending());
    manager.unload(1).await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(1), new_image)
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("extension-admission"));
    assert!(old_requests.try_recv().is_err());
    assert!(new_requests.try_recv().is_err());
    assert!(
        manager
            .get_image_bytes(1, "removed-source".into())
            .await
            .is_err()
    );
    drop((old_active, new_active));
}
