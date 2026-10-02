use std::{
    io::Cursor,
    sync::{Condvar, atomic::AtomicUsize},
};

use bytes::Bytes;
use tanoshi_lib::prelude::{Extension, Lang};
use tokio::io::{DuplexStream, duplex};

use super::*;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

struct Dispatcher {
    calls: mpsc::Sender<WorkerCall>,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
    peer: DuplexStream,
    preferences: SavedPreferences,
    health: Arc<SourceHealth>,
}

impl Dispatcher {
    fn new(capacity: usize) -> Self {
        Self::with_buffer_size(capacity, 256)
    }

    fn with_buffer_size(capacity: usize, buffer_size: usize) -> Self {
        // Small buffers force framing to survive interleaved transport work.
        let (host, peer) = duplex(buffer_size);
        let (reader, writer) = tokio::io::split(host);
        Self::with_transport(capacity, reader, writer, peer)
    }

    fn with_transport<R, W>(capacity: usize, reader: R, writer: W, peer: DuplexStream) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (calls, requests) = mpsc::channel(capacity);
        let (shutdown, stopped) = watch::channel(false);
        let preferences = Arc::new(StdMutex::new(None));
        let health = SourceHealth::new();
        let task = tokio::spawn(dispatch_requests(
            writer,
            reader,
            DispatcherState {
                requests,
                shutdown: stopped,
                preferences: preferences.clone(),
                health: health.clone(),
                next_request_id: 1,
                max_concurrent_calls: capacity,
            },
        ));
        Self {
            calls,
            shutdown,
            task,
            peer,
            preferences,
            health,
        }
    }

    async fn enqueue(
        &self,
        request: WorkerRequest,
        timeout: Duration,
    ) -> oneshot::Receiver<WorkerReply> {
        let (reply, response) = oneshot::channel();
        self.calls
            .send(WorkerCall {
                request,
                deadline: Instant::now() + timeout,
                reply,
            })
            .await
            .unwrap();
        response
    }

    async fn pages(&self, path: &str, timeout: Duration) -> oneshot::Receiver<WorkerReply> {
        self.enqueue(
            WorkerRequest::GetPages {
                path: path.to_owned(),
            },
            timeout,
        )
        .await
    }

    async fn next_request(&mut self) -> WorkerRequestEnvelope {
        tokio::time::timeout(TEST_TIMEOUT, read_frame_async(&mut self.peer))
            .await
            .unwrap()
            .unwrap()
    }

    async fn respond(&mut self, id: u64, value: WorkerValue) {
        write_frame_async(&mut self.peer, &WorkerResponse::Result { id, value })
            .await
            .unwrap();
    }

    async fn finish(&mut self) {
        tokio::time::timeout(TEST_TIMEOUT, &mut self.task)
            .await
            .unwrap()
            .unwrap();
    }
}

impl Drop for Dispatcher {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn response(reply: oneshot::Receiver<WorkerReply>) -> WorkerResult {
    let reply = tokio::time::timeout(TEST_TIMEOUT, reply)
        .await
        .unwrap()
        .unwrap();
    match reply {
        WorkerReply::Finished(result) => result,
        WorkerReply::Retry(request) => panic!("unexpected undispatched request: {request:?}"),
    }
}

fn assert_pages(value: WorkerResult, path: &str) {
    assert!(matches!(value, Ok(WorkerValue::Pages(pages)) if pages == [path]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_expired_unwritten_request_is_discarded_and_keeps_the_worker_running() {
    let request = WorkerRequestEnvelope {
        id: 1,
        request: WorkerRequest::GetPages {
            path: "first".into(),
        },
    };
    let frame_size = serialize_frame(&request).unwrap().len();
    // The first complete frame fills the pipe, so the second cannot write a byte.
    let mut dispatcher = Dispatcher::with_buffer_size(3, frame_size);
    let first = dispatcher.pages("first", TEST_TIMEOUT).await;
    let expired = dispatcher.pages("expired", Duration::from_millis(50)).await;
    assert!(matches!(
        response(expired).await,
        Err(WorkerCallError::QueueTimeout)
    ));
    assert!(!dispatcher.calls.is_closed());
    let a = dispatcher.next_request().await;
    assert!(matches!(a.request, WorkerRequest::GetPages { ref path } if path == "first"));
    let fresh = dispatcher.pages("fresh", TEST_TIMEOUT).await;
    let b = dispatcher.next_request().await;
    assert!(matches!(b.request, WorkerRequest::GetPages { ref path } if path == "fresh"));
    dispatcher
        .respond(a.id, WorkerValue::Pages(vec!["first".into()]))
        .await;
    dispatcher
        .respond(b.id, WorkerValue::Pages(vec!["fresh".into()]))
        .await;
    assert_pages(response(first).await, "first");
    assert_pages(response(fresh).await, "fresh");
    assert!(!dispatcher.health.record_failure());
    assert!(!dispatcher.health.record_failure());
    assert!(dispatcher.health.record_failure());
}

#[tokio::test]
async fn expired_requests_behind_a_partial_write_never_reach_the_peer() {
    let mut dispatcher = Dispatcher::with_buffer_size(3, 8);
    let first = dispatcher.pages("first", TEST_TIMEOUT).await;
    let expired = dispatcher.pages("expired", Duration::from_millis(50)).await;
    assert!(matches!(
        response(expired).await,
        Err(WorkerCallError::QueueTimeout)
    ));
    assert!(!dispatcher.calls.is_closed());
    let a = dispatcher.next_request().await;
    let fresh = dispatcher.pages("fresh", TEST_TIMEOUT).await;
    let b = dispatcher.next_request().await;
    assert!(matches!(b.request, WorkerRequest::GetPages { ref path } if path == "fresh"));
    dispatcher
        .respond(a.id, WorkerValue::Pages(vec!["first".into()]))
        .await;
    dispatcher
        .respond(b.id, WorkerValue::Pages(vec!["fresh".into()]))
        .await;
    assert_pages(response(first).await, "first");
    assert_pages(response(fresh).await, "fresh");
}

#[tokio::test]
async fn repeated_expirations_reclaim_writer_capacity_without_a_false_crash() {
    let mut dispatcher = Dispatcher::with_buffer_size(2, 8);
    let first = dispatcher.pages("blocked", TEST_TIMEOUT).await;
    for index in 0..6 {
        let expired = dispatcher
            .pages(&format!("expired{index}"), Duration::from_millis(25))
            .await;
        assert!(matches!(
            response(expired).await,
            Err(WorkerCallError::QueueTimeout)
        ));
        assert!(!dispatcher.calls.is_closed());
    }
    let fresh = dispatcher.pages("fresh", TEST_TIMEOUT).await;
    let a = dispatcher.next_request().await;
    let b = dispatcher.next_request().await;
    assert!(matches!(a.request, WorkerRequest::GetPages { ref path } if path == "blocked"));
    assert!(matches!(b.request, WorkerRequest::GetPages { ref path } if path == "fresh"));
    dispatcher
        .respond(a.id, WorkerValue::Pages(vec!["blocked".into()]))
        .await;
    dispatcher
        .respond(b.id, WorkerValue::Pages(vec!["fresh".into()]))
        .await;
    assert_pages(response(first).await, "blocked");
    assert_pages(response(fresh).await, "fresh");
    assert!(!dispatcher.health.record_failure());
    assert!(!dispatcher.health.record_failure());
    assert!(dispatcher.health.record_failure());
}

#[tokio::test]
async fn a_full_live_writer_queue_is_backpressure_not_a_crash() {
    let mut dispatcher = Dispatcher::with_buffer_size(1, 8);
    let first = dispatcher.pages("blocked", TEST_TIMEOUT).await;
    // Ensure the first frame is held by the writer rather than its queue.
    let mut prefix = [0; 4];
    dispatcher.peer.read_exact(&mut prefix).await.unwrap();
    let queued = dispatcher.pages("queued", TEST_TIMEOUT).await;
    let full = dispatcher.pages("full", TEST_TIMEOUT).await;
    assert!(matches!(
        response(full).await,
        Err(WorkerCallError::QueueTimeout)
    ));
    assert!(!dispatcher.calls.is_closed());
    let mut body = vec![0; u32::from_be_bytes(prefix) as usize];
    dispatcher.peer.read_exact(&mut body).await.unwrap();
    let a: WorkerRequestEnvelope = serde_json::from_slice(&body).unwrap();
    let b = dispatcher.next_request().await;
    assert!(matches!(b.request, WorkerRequest::GetPages { ref path } if path == "queued"));
    dispatcher
        .respond(a.id, WorkerValue::Pages(vec!["blocked".into()]))
        .await;
    dispatcher
        .respond(b.id, WorkerValue::Pages(vec!["queued".into()]))
        .await;
    assert_pages(response(first).await, "blocked");
    assert_pages(response(queued).await, "queued");
}

#[tokio::test]
async fn an_unwritten_call_interrupted_by_a_crash_keeps_its_retry_allowance() {
    let envelope = WorkerRequestEnvelope {
        id: 1,
        request: WorkerRequest::GetPages {
            path: "sent".into(),
        },
    };
    let mut dispatcher = Dispatcher::with_buffer_size(2, serialize_frame(&envelope).unwrap().len());
    let sent = dispatcher.pages("sent", TEST_TIMEOUT).await;
    let unwritten = dispatcher.pages("unwritten", TEST_TIMEOUT).await;
    // Let the writer fill the pipe with the first complete frame.
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    dispatcher.peer.shutdown().await.unwrap();
    assert!(matches!(
        response(sent).await,
        Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::Crash,
            ..
        })
    ));
    assert!(matches!(
        response(unwritten).await,
        Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::NotDispatched,
            ..
        })
    ));
    dispatcher.finish().await;
}

struct DelayedFlush<W>(W);

impl<W: AsyncWrite + Unpin> AsyncWrite for DelayedFlush<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut TaskContext<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_write(context, bytes)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        // The surrounding write timeout wakes this future at the deadline.
        Poll::Pending
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_shutdown(context)
    }
}

#[tokio::test]
async fn a_complete_frame_with_a_late_flush_still_allows_peers_to_drain() {
    let (host, peer) = duplex(256);
    let (reader, writer) = tokio::io::split(host);
    let mut dispatcher = Dispatcher::with_transport(2, reader, DelayedFlush(writer), peer);
    let expired = dispatcher.pages("expired", Duration::from_millis(50)).await;
    let healthy = dispatcher.pages("healthy", TEST_TIMEOUT).await;
    let first = dispatcher.next_request().await;
    assert_eq!(first.id, 1);
    assert!(matches!(
        response(expired).await,
        Err(WorkerCallError::Timeout)
    ));
    let second = dispatcher.next_request().await;
    dispatcher
        .respond(second.id, WorkerValue::Pages(vec!["healthy".into()]))
        .await;
    assert_pages(response(healthy).await, "healthy");
    dispatcher.finish().await;
}

#[tokio::test]
async fn the_writer_rechecks_quarantine_before_the_first_byte() {
    let envelope = WorkerRequestEnvelope {
        id: 1,
        request: WorkerRequest::GetPages {
            path: "first".into(),
        },
    };
    let (host, mut peer) = duplex(serialize_frame(&envelope).unwrap().len());
    let outgoing = WriteQueue::new(2);
    let (events, mut received) = mpsc::channel(2);
    let health = SourceHealth::new();
    let writer = tokio::spawn(write_requests(
        host,
        outgoing.clone(),
        events,
        health.clone(),
    ));
    let progress = Arc::new(StdMutex::new(WriteProgress::default()));
    outgoing
        .push(OutgoingCall {
            envelope,
            deadline: Instant::now() + TEST_TIMEOUT,
            progress: progress.clone(),
        })
        .unwrap();
    tokio::time::timeout(TEST_TIMEOUT, async {
        while !progress.lock().unwrap().complete {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    outgoing
        .push(OutgoingCall {
            envelope: WorkerRequestEnvelope {
                id: 2,
                request: WorkerRequest::GetPages {
                    path: "blocked".into(),
                },
            },
            deadline: Instant::now() + TEST_TIMEOUT,
            progress: Arc::new(StdMutex::new(WriteProgress::default())),
        })
        .unwrap();
    tokio::task::yield_now().await;
    health.quarantine();
    let first: WorkerRequestEnvelope = read_frame_async(&mut peer).await.unwrap();
    assert_eq!(first.id, 1);
    assert!(matches!(
        tokio::time::timeout(TEST_TIMEOUT, received.recv())
            .await
            .unwrap(),
        Some(TransportEvent::Rejected {
            id: 2,
            admission: SourceAdmission::Quarantined
        })
    ));
    outgoing.close();
    writer.await.unwrap();
    let mut remaining = Vec::new();
    peer.read_to_end(&mut remaining).await.unwrap();
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn a_partial_frame_timeout_stops_the_transport_without_a_drain_delay() {
    let mut dispatcher = Dispatcher::with_buffer_size(2, 8);
    let first = dispatcher.pages("first", Duration::from_millis(50)).await;
    let peer = dispatcher.pages("peer", TEST_TIMEOUT).await;
    let mut prefix = [0; 4];
    dispatcher.peer.read_exact(&mut prefix).await.unwrap();
    let start = Instant::now();
    assert!(matches!(
        response(first).await,
        Err(WorkerCallError::Timeout)
    ));
    assert!(matches!(
        response(peer).await,
        Err(WorkerCallError::Restarted { .. })
    ));
    dispatcher.finish().await;
    assert!(start.elapsed() < TIMEOUT_DRAIN_GRACE);
    let mut remaining = Vec::new();
    dispatcher.peer.read_to_end(&mut remaining).await.unwrap();
    assert!(remaining.len() < u32::from_be_bytes(prefix) as usize);
    assert!(!dispatcher.health.record_failure());
    assert!(!dispatcher.health.record_failure());
    assert!(dispatcher.health.record_failure());
}

#[tokio::test]
async fn out_of_order_replies_and_partial_frames_keep_their_request_ids() {
    let mut dispatcher = Dispatcher::new(4);
    let first = dispatcher.pages("first", TEST_TIMEOUT).await;
    let second = dispatcher.pages("second", TEST_TIMEOUT).await;
    let third = dispatcher.pages("third", TEST_TIMEOUT).await;
    let a = dispatcher.next_request().await;
    let b = dispatcher.next_request().await;
    let c = dispatcher.next_request().await;

    let partial = serialize_frame(&WorkerResponse::Result {
        id: c.id,
        value: WorkerValue::Pages(vec!["third".into()]),
    })
    .unwrap();
    dispatcher.peer.write_all(&partial[..6]).await.unwrap();
    let fourth = dispatcher.pages("fourth", TEST_TIMEOUT).await;
    let d = dispatcher.next_request().await;
    dispatcher.peer.write_all(&partial[6..]).await.unwrap();
    assert_pages(response(third).await, "third");

    dispatcher
        .respond(b.id, WorkerValue::Pages(vec!["second".into()]))
        .await;
    dispatcher
        .respond(d.id, WorkerValue::Pages(vec!["fourth".into()]))
        .await;
    dispatcher
        .respond(a.id, WorkerValue::Pages(vec!["first".into()]))
        .await;
    assert_pages(response(first).await, "first");
    assert_pages(response(second).await, "second");
    assert_pages(response(fourth).await, "fourth");
}

#[tokio::test]
async fn timeout_retires_the_worker_and_allows_healthy_peers_to_finish() {
    let mut dispatcher = Dispatcher::new(2);
    let expired = dispatcher
        .pages("expired", Duration::from_millis(150))
        .await;
    let peer = dispatcher.pages("peer", TEST_TIMEOUT).await;
    let a = dispatcher.next_request().await;
    let b = dispatcher.next_request().await;

    assert!(matches!(
        response(expired).await,
        Err(WorkerCallError::Timeout)
    ));
    assert!(dispatcher.calls.is_closed());
    // A late reply is recognized and drained rather than mistaken for a peer.
    dispatcher
        .respond(a.id, WorkerValue::Pages(vec!["late".into()]))
        .await;
    dispatcher
        .respond(b.id, WorkerValue::Pages(vec!["peer".into()]))
        .await;
    assert_pages(response(peer).await, "peer");
    dispatcher.finish().await;
}

#[tokio::test]
async fn retiring_worker_keeps_each_peers_deadline() {
    let mut dispatcher = Dispatcher::new(2);
    let first = dispatcher.pages("first", Duration::from_millis(100)).await;
    let mut second = dispatcher.pages("second", Duration::from_millis(250)).await;
    dispatcher.next_request().await;
    dispatcher.next_request().await;
    assert!(matches!(
        response(first).await,
        Err(WorkerCallError::Timeout)
    ));
    assert!(matches!(
        second.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    assert!(matches!(
        response(second).await,
        Err(WorkerCallError::Timeout)
    ));
    dispatcher.finish().await;
}

#[tokio::test]
async fn timeout_recovery_does_not_wait_for_a_long_image_deadline() {
    let mut dispatcher = Dispatcher::new(2);
    let expired = dispatcher.pages("expired", Duration::from_millis(50)).await;
    let image = dispatcher
        .enqueue(
            WorkerRequest::GetImageBytes {
                url: "slow-image".into(),
            },
            Duration::from_secs(120),
        )
        .await;
    dispatcher.next_request().await;
    dispatcher.next_request().await;
    assert!(matches!(
        response(expired).await,
        Err(WorkerCallError::Timeout)
    ));
    let start = Instant::now();
    assert!(matches!(
        response(image).await,
        Err(WorkerCallError::Restarted { .. })
    ));
    assert!(start.elapsed() < TIMEOUT_DRAIN_GRACE + Duration::from_millis(500));
    dispatcher.finish().await;
    assert_eq!(
        dispatcher.health.admission(),
        super::super::source::SourceAdmission::Allowed
    );
}

#[tokio::test]
async fn calls_already_queued_when_a_worker_stops_are_returned_for_retry() {
    let mut dispatcher = Dispatcher::new(2);
    // Enqueue without yielding, then stop before the dispatcher can consume.
    let queued = dispatcher.pages("queued", TEST_TIMEOUT).await;
    dispatcher.shutdown.send_replace(true);
    // Closed stdin makes either ready select branch retire the old process.
    dispatcher.peer.shutdown().await.unwrap();
    let reply = tokio::time::timeout(TEST_TIMEOUT, queued)
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(reply, WorkerReply::Retry(WorkerRequest::GetPages { path }) if path == "queued")
    );
    dispatcher.finish().await;
}

#[tokio::test]
async fn a_timeout_retries_calls_already_in_the_host_queue() {
    use std::{
        future::Future,
        task::{Context, Waker},
    };
    let (host, _peer) = duplex(256);
    let (reader, writer) = tokio::io::split(host);
    let (calls, incoming) = mpsc::channel(2);
    let (_shutdown, stopped) = watch::channel(false);
    let (reply, expired) = oneshot::channel();
    calls
        .try_send(WorkerCall {
            request: WorkerRequest::GetPages {
                path: "expired".into(),
            },
            deadline: Instant::now() + Duration::from_millis(20),
            reply,
        })
        .unwrap();
    let mut dispatcher = Box::pin(dispatch_requests(
        writer,
        reader,
        DispatcherState {
            requests: incoming,
            shutdown: stopped,
            preferences: Arc::new(StdMutex::new(None)),
            health: SourceHealth::new(),
            next_request_id: 1,
            max_concurrent_calls: 2,
        },
    ));
    // Drive the supervisor explicitly so it cannot race the queue setup.
    assert!(
        dispatcher
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    tokio::time::sleep(Duration::from_millis(30)).await;
    let (reply, queued) = oneshot::channel();
    calls
        .try_send(WorkerCall {
            request: WorkerRequest::GetPages {
                path: "queued".into(),
            },
            deadline: Instant::now() + TEST_TIMEOUT,
            reply,
        })
        .unwrap();
    assert!(
        dispatcher
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_ready()
    );
    assert!(matches!(
        response(expired).await,
        Err(WorkerCallError::Timeout)
    ));
    assert!(
        matches!(queued.await.unwrap(), WorkerReply::Retry(WorkerRequest::GetPages { path }) if path == "queued")
    );
}

#[tokio::test]
async fn retiring_also_retries_senders_that_reserved_capacity_before_close() {
    let mut dispatcher = Dispatcher::new(2);
    let reserved = dispatcher.calls.clone().reserve_owned().await.unwrap();
    dispatcher.shutdown.send_replace(true);
    dispatcher.calls.closed().await;
    let (reply, queued) = oneshot::channel();
    reserved.send(WorkerCall {
        request: WorkerRequest::GetPages {
            path: "reserved".into(),
        },
        deadline: Instant::now() + TEST_TIMEOUT,
        reply,
    });
    assert!(
        matches!(queued.await.unwrap(), WorkerReply::Retry(WorkerRequest::GetPages { path }) if path == "reserved")
    );
    dispatcher.finish().await;
}

#[tokio::test]
async fn shutdown_releases_all_waiters_during_a_partial_reply() {
    let mut dispatcher = Dispatcher::new(2);
    let a = dispatcher.pages("first", TEST_TIMEOUT).await;
    let b = dispatcher.pages("second", TEST_TIMEOUT).await;
    let first = dispatcher.next_request().await;
    dispatcher.next_request().await;
    let frame = serialize_frame(&WorkerResponse::Result {
        id: first.id,
        value: WorkerValue::Pages(vec!["first".into()]),
    })
    .unwrap();
    dispatcher.peer.write_all(&frame[..6]).await.unwrap();
    dispatcher.shutdown.send_replace(true);
    assert!(matches!(response(a).await, Err(WorkerCallError::Stopped)));
    assert!(matches!(response(b).await, Err(WorkerCallError::Stopped)));
    dispatcher.finish().await;
}

#[tokio::test]
async fn unattributed_crash_counts_once_without_blaming_the_oldest_call() {
    let mut dispatcher = Dispatcher::new(3);
    let a = dispatcher
        .enqueue(
            WorkerRequest::SetPreferences {
                preferences: vec![],
            },
            TEST_TIMEOUT,
        )
        .await;
    let b = dispatcher.pages("second", TEST_TIMEOUT).await;
    let c = dispatcher.pages("third", TEST_TIMEOUT).await;
    for _ in 0..3 {
        dispatcher.next_request().await;
    }
    dispatcher.peer.shutdown().await.unwrap();
    let results = [response(a).await, response(b).await, response(c).await];
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(WorkerCallError::Restarted { .. })))
            .count(),
        3
    );
    assert_eq!(
        dispatcher.health.admission(),
        super::super::source::SourceAdmission::Allowed
    );
    // Two more failures reach the threshold, proving the crash added one.
    assert!(!dispatcher.health.record_failure());
    assert!(dispatcher.health.record_failure());
    dispatcher.finish().await;
}

#[tokio::test]
async fn idle_worker_crashes_do_not_count_toward_quarantine() {
    let mut dispatcher = Dispatcher::new(1);
    dispatcher.peer.shutdown().await.unwrap();
    dispatcher.finish().await;
    assert!(!dispatcher.health.record_failure());
    assert!(!dispatcher.health.record_failure());
    assert!(dispatcher.health.record_failure());
}

#[tokio::test]
async fn unknown_response_id_fails_waiters_without_delivering_wrong_data() {
    let mut dispatcher = Dispatcher::new(2);
    let a = dispatcher.pages("first", TEST_TIMEOUT).await;
    let b = dispatcher.pages("second", TEST_TIMEOUT).await;
    dispatcher.next_request().await;
    dispatcher.next_request().await;
    dispatcher
        .respond(99, WorkerValue::Pages(vec!["wrong".into()]))
        .await;
    assert!(response(a).await.is_err());
    assert!(response(b).await.is_err());
    dispatcher.finish().await;
}

#[tokio::test]
async fn cancelled_response_subscription_does_not_desynchronize_peers() {
    let mut dispatcher = Dispatcher::new(2);
    let cancelled = dispatcher.pages("cancelled", TEST_TIMEOUT).await;
    let peer = dispatcher.pages("peer", TEST_TIMEOUT).await;
    let a = dispatcher.next_request().await;
    let b = dispatcher.next_request().await;
    drop(cancelled);
    dispatcher
        .respond(a.id, WorkerValue::Pages(vec!["cancelled".into()]))
        .await;
    dispatcher
        .respond(b.id, WorkerValue::Pages(vec!["peer".into()]))
        .await;
    assert_pages(response(peer).await, "peer");
}

#[tokio::test]
async fn acknowledged_preferences_are_saved_before_the_reply() {
    let mut dispatcher = Dispatcher::new(1);
    let updated = vec![Input::Text {
        name: "token".into(),
        state: Some("updated".into()),
    }];
    let reply = dispatcher
        .enqueue(
            WorkerRequest::SetPreferences {
                preferences: updated.clone(),
            },
            TEST_TIMEOUT,
        )
        .await;
    let request = dispatcher.next_request().await;
    dispatcher.respond(request.id, WorkerValue::Unit).await;
    assert!(matches!(response(reply).await, Ok(WorkerValue::Unit)));
    assert_eq!(
        serde_json::to_value(dispatcher.preferences.lock().unwrap().as_ref().unwrap()).unwrap(),
        serde_json::to_value(updated).unwrap(),
    );
}

#[tokio::test]
async fn expired_call_is_rejected_before_any_worker_dispatch() {
    let mut dispatcher = Dispatcher::new(1);
    let (reply, expired) = oneshot::channel();
    dispatcher
        .calls
        .send(WorkerCall {
            request: WorkerRequest::GetPages {
                path: "expired".into(),
            },
            deadline: Instant::now() - Duration::from_secs(1),
            reply,
        })
        .await
        .unwrap();
    assert!(matches!(
        response(expired).await,
        Err(WorkerCallError::QueueTimeout)
    ));
    let fresh = dispatcher.pages("fresh", TEST_TIMEOUT).await;
    let request = dispatcher.next_request().await;
    assert!(matches!(request.request, WorkerRequest::GetPages { path } if path == "fresh"));
    dispatcher
        .respond(request.id, WorkerValue::Pages(vec!["fresh".into()]))
        .await;
    assert_pages(response(fresh).await, "fresh");
}

#[tokio::test]
async fn operation_error_and_panic_do_not_change_peer_results() {
    let mut dispatcher = Dispatcher::new(3);
    let operation = dispatcher.pages("error", TEST_TIMEOUT).await;
    let panic = dispatcher.pages("panic", TEST_TIMEOUT).await;
    let healthy = dispatcher.pages("healthy", TEST_TIMEOUT).await;
    let a = dispatcher.next_request().await;
    let b = dispatcher.next_request().await;
    let c = dispatcher.next_request().await;
    for (id, kind) in [
        (a.id, WorkerErrorKind::Operation),
        (b.id, WorkerErrorKind::Panic),
    ] {
        write_frame_async(
            &mut dispatcher.peer,
            &WorkerResponse::Error {
                id,
                kind,
                message: "fixture error".into(),
            },
        )
        .await
        .unwrap();
    }
    dispatcher
        .respond(c.id, WorkerValue::Pages(vec!["healthy".into()]))
        .await;
    assert!(matches!(
        response(operation).await,
        Err(WorkerCallError::Remote {
            kind: WorkerErrorKind::Operation,
            ..
        })
    ));
    assert!(matches!(
        response(panic).await,
        Err(WorkerCallError::Remote {
            kind: WorkerErrorKind::Panic,
            ..
        })
    ));
    assert_pages(response(healthy).await, "healthy");
}

#[tokio::test]
async fn large_image_replies_and_inline_metadata_replies_share_the_transport() {
    let mut dispatcher = Dispatcher::new(2);
    let image = dispatcher
        .enqueue(
            WorkerRequest::GetImageBytes {
                url: "image".into(),
            },
            TEST_TIMEOUT,
        )
        .await;
    let metadata = dispatcher.pages("metadata", TEST_TIMEOUT).await;
    let first = dispatcher.next_request().await;
    let second = dispatcher.next_request().await;
    let bytes = vec![123; INLINE_RESPONSE_LIMIT * 2];
    dispatcher
        .respond(
            first.id,
            WorkerValue::Image {
                bytes: bytes.clone(),
            },
        )
        .await;
    dispatcher
        .respond(second.id, WorkerValue::Pages(vec!["metadata".into()]))
        .await;
    assert!(
        matches!(response(image).await, Ok(WorkerValue::Image { bytes: actual }) if actual == bytes)
    );
    assert_pages(response(metadata).await, "metadata");
}

async fn test_client() -> (Arc<WorkerClient>, mpsc::Receiver<WorkerCall>) {
    let client = WorkerClient::new(PathBuf::new(), PathBuf::new(), TEST_TIMEOUT, 8, None);
    let (requests, incoming) = mpsc::channel(8);
    let (shutdown, _) = watch::channel(false);
    *client.process.lock().await = Some(WorkerProcess {
        requests,
        shutdown,
        task: None,
        source_info: WorkerSourceInfo {
            id: 1,
            name: "test".into(),
            url: String::new(),
            version: "test".into(),
            icon: String::new(),
            languages: Lang::All,
            nsfw: false,
        },
        rustc_version: String::new(),
        lib_version: String::new(),
    });
    (client, incoming)
}

fn request_task(
    client: Arc<WorkerClient>,
    request: WorkerRequest,
    timeout: Duration,
) -> JoinHandle<WorkerResult> {
    tokio::spawn(async move { client.request(request, timeout).await })
}

async fn next_call(incoming: &mut mpsc::Receiver<WorkerCall>) -> WorkerCall {
    tokio::time::timeout(TEST_TIMEOUT, incoming.recv())
        .await
        .unwrap()
        .unwrap()
}

async fn wait_for_writer(client: &WorkerClient) {
    tokio::time::timeout(TEST_TIMEOUT, async {
        while client.preference_gate.try_read().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn preferences_wait_in_the_host_and_hold_back_new_reads() {
    let (client, mut incoming) = test_client().await;
    let image = request_task(
        client.clone(),
        WorkerRequest::GetImageBytes {
            url: "image".into(),
        },
        TEST_TIMEOUT,
    );
    let running = next_call(&mut incoming).await;
    let preferences = request_task(
        client.clone(),
        WorkerRequest::SetPreferences {
            preferences: vec![],
        },
        TEST_TIMEOUT,
    );
    wait_for_writer(&client).await;
    let reader = request_task(
        client.clone(),
        WorkerRequest::GetPages { path: "new".into() },
        TEST_TIMEOUT,
    );
    tokio::task::yield_now().await;
    assert!(matches!(
        incoming.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));

    running
        .reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Image {
            bytes: vec![],
        })))
        .unwrap();
    image.await.unwrap().unwrap();
    let writer = next_call(&mut incoming).await;
    assert!(matches!(
        writer.request,
        WorkerRequest::SetPreferences { .. }
    ));
    assert!(matches!(
        incoming.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    writer
        .reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Unit)))
        .unwrap();
    preferences.await.unwrap().unwrap();
    let read = next_call(&mut incoming).await;
    assert!(matches!(read.request, WorkerRequest::GetPages { ref path } if path == "new"));
    read.reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Pages(vec![
            "new".into(),
        ]))))
        .unwrap();
    assert_pages(reader.await.unwrap(), "new");
}

#[tokio::test]
async fn a_preference_queue_timeout_leaves_the_running_worker_available() {
    let (client, mut incoming) = test_client().await;
    let image = request_task(
        client.clone(),
        WorkerRequest::GetImageBytes {
            url: "image".into(),
        },
        Duration::from_secs(120),
    );
    let running = next_call(&mut incoming).await;
    let preferences = request_task(
        client.clone(),
        WorkerRequest::SetPreferences {
            preferences: vec![],
        },
        Duration::from_millis(50),
    );
    wait_for_writer(&client).await;
    assert!(matches!(
        preferences.await.unwrap(),
        Err(WorkerCallError::QueueTimeout)
    ));
    assert!(matches!(
        incoming.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert!(!incoming.is_closed());
    assert_eq!(
        client.health.admission(),
        super::super::source::SourceAdmission::Allowed
    );
    let reader = request_task(
        client,
        WorkerRequest::GetPages {
            path: "after".into(),
        },
        TEST_TIMEOUT,
    );
    let read = next_call(&mut incoming).await;
    read.reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Pages(vec![
            "after".into(),
        ]))))
        .unwrap();
    assert_pages(reader.await.unwrap(), "after");
    assert!(!image.is_finished());
    running
        .reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Image {
            bytes: vec![],
        })))
        .unwrap();
    image.await.unwrap().unwrap();
}

#[tokio::test]
async fn shutdown_releases_a_preference_writer_waiting_in_the_host() {
    let (client, mut incoming) = test_client().await;
    let image = request_task(
        client.clone(),
        WorkerRequest::GetImageBytes {
            url: "image".into(),
        },
        TEST_TIMEOUT,
    );
    let _running = next_call(&mut incoming).await;
    let writer = request_task(
        client.clone(),
        WorkerRequest::SetPreferences {
            preferences: vec![],
        },
        TEST_TIMEOUT,
    );
    wait_for_writer(&client).await;
    tokio::time::timeout(TEST_TIMEOUT, client.pause())
        .await
        .unwrap();
    assert!(matches!(
        image.await.unwrap(),
        Err(WorkerCallError::Stopped)
    ));
    assert!(matches!(
        writer.await.unwrap(),
        Err(WorkerCallError::Stopped)
    ));
}

#[tokio::test]
async fn an_undispatched_retry_preserves_the_original_request() {
    let (client, mut incoming) = test_client().await;
    let reader = request_task(
        client,
        WorkerRequest::GetPages {
            path: "retry".into(),
        },
        TEST_TIMEOUT,
    );
    let queued = next_call(&mut incoming).await;
    let deadline = queued.deadline;
    queued
        .reply
        .send(WorkerReply::Retry(queued.request))
        .unwrap();
    let retry = next_call(&mut incoming).await;
    assert_eq!(retry.deadline, deadline);
    assert!(matches!(retry.request, WorkerRequest::GetPages { ref path } if path == "retry"));
    retry
        .reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Pages(vec![
            "retry".into(),
        ]))))
        .unwrap();
    assert_pages(reader.await.unwrap(), "retry");
}

#[tokio::test]
async fn an_interrupted_read_is_retried_with_its_original_deadline() {
    let (client, mut incoming) = test_client().await;
    let reader = request_task(
        client,
        WorkerRequest::GetPages {
            path: "retry".into(),
        },
        TEST_TIMEOUT,
    );
    let interrupted = next_call(&mut incoming).await;
    let deadline = interrupted.deadline;
    interrupted
        .reply
        .send(WorkerReply::Finished(Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::TimeoutRecovery,
            message: "fixture restart".into(),
        })))
        .unwrap();
    let retried = next_call(&mut incoming).await;
    assert_eq!(retried.deadline, deadline);
    assert!(matches!(retried.request, WorkerRequest::GetPages { ref path } if path == "retry"));
    retried
        .reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Pages(vec![
            "retry".into(),
        ]))))
        .unwrap();
    assert_pages(reader.await.unwrap(), "retry");
}

#[tokio::test]
async fn interrupted_preferences_are_reapplied_with_the_same_values() {
    let (client, mut incoming) = test_client().await;
    let preferences = vec![Input::Text {
        name: "token".into(),
        state: Some("value".into()),
    }];
    let writer = request_task(
        client,
        WorkerRequest::SetPreferences {
            preferences: preferences.clone(),
        },
        TEST_TIMEOUT,
    );
    let interrupted = next_call(&mut incoming).await;
    let deadline = interrupted.deadline;
    interrupted
        .reply
        .send(WorkerReply::Finished(Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::TimeoutRecovery,
            message: "fixture restart".into(),
        })))
        .unwrap();
    let retried = next_call(&mut incoming).await;
    assert_eq!(retried.deadline, deadline);
    let WorkerRequest::SetPreferences {
        preferences: updated,
    } = retried.request
    else {
        panic!("unexpected retry")
    };
    assert_eq!(
        serde_json::to_value(updated).unwrap(),
        serde_json::to_value(preferences).unwrap()
    );
    retried
        .reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Unit)))
        .unwrap();
    assert!(matches!(writer.await.unwrap(), Ok(WorkerValue::Unit)));
}

#[tokio::test]
async fn retries_stop_at_quarantine_without_a_fourth_dispatch() {
    for dispatched in [false, true] {
        let (client, mut incoming) = test_client().await;
        let reader = request_task(
            client.clone(),
            WorkerRequest::GetPages {
                path: "crash".into(),
            },
            TEST_TIMEOUT,
        );
        let failures = if dispatched {
            // A previously recorded failure leaves two dispatches before
            // quarantine, matching the per-call crash retry budget.
            assert!(!client.health.record_failure());
            2..=3
        } else {
            1..=3
        };
        for failure in failures {
            let attempt = next_call(&mut incoming).await;
            assert_eq!(client.health.record_failure(), failure == 3);
            let reply = if dispatched {
                WorkerReply::Finished(Err(WorkerCallError::Restarted {
                    reason: WorkerRestartReason::Crash,
                    message: "fixture crash".into(),
                }))
            } else {
                WorkerReply::Retry(attempt.request)
            };
            attempt.reply.send(reply).unwrap();
        }
        assert!(matches!(
            reader.await.unwrap(),
            Err(WorkerCallError::Admission(SourceAdmission::Quarantined))
        ));
        assert!(matches!(
            incoming.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }
}

#[tokio::test]
async fn successful_peers_do_not_reset_the_crashing_calls_retry_budget() {
    let (client, mut incoming) = test_client().await;
    let bad = request_task(
        client.clone(),
        WorkerRequest::GetImageBytes { url: "bad".into() },
        Duration::from_secs(120),
    );
    let first = next_call(&mut incoming).await;
    let deadline = first.deadline;
    assert!(!client.health.record_failure());
    first
        .reply
        .send(WorkerReply::Finished(Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::Crash,
            message: "first crash".into(),
        })))
        .unwrap();
    let retried = next_call(&mut incoming).await;
    assert_eq!(retried.deadline, deadline);
    assert!(matches!(retried.request, WorkerRequest::GetImageBytes { ref url } if url == "bad"));

    let peer = request_task(
        client.clone(),
        WorkerRequest::GetPages {
            path: "healthy".into(),
        },
        TEST_TIMEOUT,
    );
    let healthy = next_call(&mut incoming).await;
    healthy
        .reply
        .send(WorkerReply::Finished(Ok(WorkerValue::Pages(vec![
            "healthy".into(),
        ]))))
        .unwrap();
    assert_pages(peer.await.unwrap(), "healthy");
    // The manager records this after decoding the successful peer's reply.
    client.health.record_success();
    assert!(!client.health.record_failure());
    retried
        .reply
        .send(WorkerReply::Finished(Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::Crash,
            message: "second crash".into(),
        })))
        .unwrap();

    assert!(matches!(
        tokio::time::timeout(TEST_TIMEOUT, bad)
            .await
            .unwrap()
            .unwrap(),
        Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::Crash,
            ..
        })
    ));
    assert_eq!(client.health.admission(), SourceAdmission::Allowed);
    assert!(matches!(
        incoming.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn timeout_recovery_retries_do_not_consume_the_crash_retry_budget() {
    let (client, mut incoming) = test_client().await;
    let reader = request_task(
        client,
        WorkerRequest::GetPages {
            path: "retry".into(),
        },
        TEST_TIMEOUT,
    );
    let mut deadline = None;
    for reason in [
        WorkerRestartReason::TimeoutRecovery,
        WorkerRestartReason::TimeoutRecovery,
        WorkerRestartReason::NotDispatched,
        WorkerRestartReason::Crash,
        WorkerRestartReason::TimeoutRecovery,
        WorkerRestartReason::TimeoutRecovery,
        WorkerRestartReason::Crash,
    ] {
        let attempt = next_call(&mut incoming).await;
        assert_eq!(*deadline.get_or_insert(attempt.deadline), attempt.deadline);
        attempt
            .reply
            .send(WorkerReply::Finished(Err(WorkerCallError::Restarted {
                reason,
                message: "fixture restart".into(),
            })))
            .unwrap();
    }
    assert!(matches!(
        tokio::time::timeout(TEST_TIMEOUT, reader)
            .await
            .unwrap()
            .unwrap(),
        Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::Crash,
            ..
        })
    ));
    assert!(incoming.try_recv().is_err());
}

#[tokio::test]
async fn quarantine_while_waiting_for_retirement_prevents_a_replacement_spawn() {
    let (client, mut incoming) = test_client().await;
    let reader = request_task(
        client.clone(),
        WorkerRequest::GetPages {
            path: "retry".into(),
        },
        TEST_TIMEOUT,
    );
    let attempt = next_call(&mut incoming).await;
    let (started, retirement_started) = oneshot::channel();
    let (release, retired) = oneshot::channel();
    let health = client.health.clone();
    client.process.lock().await.as_mut().unwrap().task = Some(tokio::spawn(async move {
        started.send(()).unwrap();
        retired.await.unwrap();
        health.quarantine();
    }));
    drop(incoming);
    attempt
        .reply
        .send(WorkerReply::Retry(attempt.request))
        .unwrap();
    retirement_started.await.unwrap();
    // Ensure request() is waiting under the process lock before health changes.
    tokio::time::timeout(TEST_TIMEOUT, async {
        while client.process.try_lock().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        reader.await.unwrap(),
        Err(WorkerCallError::Admission(SourceAdmission::Quarantined))
    ));
    assert!(client.process.lock().await.is_none());
}

#[tokio::test]
async fn an_interrupted_call_does_not_extend_its_deadline() {
    let (client, mut incoming) = test_client().await;
    let reader = request_task(
        client,
        WorkerRequest::GetPages {
            path: "expired".into(),
        },
        Duration::from_millis(30),
    );
    let interrupted = next_call(&mut incoming).await;
    tokio::time::sleep_until(interrupted.deadline + Duration::from_millis(5)).await;
    interrupted
        .reply
        .send(WorkerReply::Finished(Err(WorkerCallError::Restarted {
            reason: WorkerRestartReason::TimeoutRecovery,
            message: "fixture restart".into(),
        })))
        .unwrap();
    assert!(matches!(
        reader.await.unwrap(),
        Err(WorkerCallError::Timeout)
    ));
    assert!(incoming.try_recv().is_err());
}

#[test]
fn panics_in_response_writing_escape_the_blocking_task_as_failures() {
    struct PanickingWriter;
    impl Write for PanickingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            panic!("fixture writer panic")
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let result = catch_worker_job(|| {
        write_response(
            &StdMutex::new(PanickingWriter),
            WorkerResponse::Result {
                id: 1,
                value: WorkerValue::Unit,
            },
        )
    });
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("fixture writer panic")
    );
}

#[derive(Default)]
struct NativeState {
    active: AtomicUsize,
    peak: AtomicUsize,
    starts: StdMutex<usize>,
    overlap: Condvar,
    preference_overlap: AtomicBool,
    preference_updates: AtomicUsize,
}

struct ActiveCall(Arc<NativeState>);
impl Drop for ActiveCall {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

struct NativeExtension {
    state: Arc<NativeState>,
    preference: String,
}

impl Extension for NativeExtension {
    fn get_source_info(&self) -> SourceInfo {
        SourceInfo {
            id: 1,
            name: "concurrent fixture".into(),
            url: String::new(),
            version: "test",
            icon: "",
            languages: Lang::All,
            nsfw: false,
        }
    }
    fn set_preferences(&mut self, preferences: Vec<Input>) -> Result<()> {
        if self.state.active.load(Ordering::SeqCst) != 0 {
            self.state.preference_overlap.store(true, Ordering::SeqCst);
        }
        let Input::Text {
            state: Some(value), ..
        } = &preferences[0]
        else {
            bail!("unexpected preference");
        };
        self.preference.clone_from(value);
        self.state.preference_updates.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn get_popular_manga(&self, _: i64) -> Result<Vec<MangaInfo>> {
        bail!("unused")
    }
    fn get_latest_manga(&self, _: i64) -> Result<Vec<MangaInfo>> {
        bail!("unused")
    }
    fn search_manga(
        &self,
        _: i64,
        _: Option<String>,
        _: Option<Vec<Input>>,
    ) -> Result<Vec<MangaInfo>> {
        bail!("unused")
    }
    fn get_manga_detail(&self, _: String) -> Result<MangaInfo> {
        bail!("unused")
    }
    fn get_chapters(&self, _: String) -> Result<Vec<ChapterInfo>> {
        bail!("unused")
    }
    fn get_image_bytes(&self, _: String) -> Result<Bytes> {
        bail!("unused")
    }
    fn get_pages(&self, path: String) -> Result<Vec<String>> {
        let active = self.state.active.fetch_add(1, Ordering::SeqCst) + 1;
        let _active = ActiveCall(self.state.clone());
        self.state.peak.fetch_max(active, Ordering::SeqCst);
        if path.starts_with("overlap") {
            let mut starts = self.state.starts.lock().unwrap();
            *starts += 1;
            self.state.overlap.notify_all();
            let (starts, _) = self
                .state
                .overlap
                .wait_timeout_while(starts, TEST_TIMEOUT, |starts| *starts < 2)
                .unwrap();
            assert!(*starts >= 2, "native extension calls were serialized");
        }
        std::thread::sleep(Duration::from_millis(5));
        Ok(vec![path, self.preference.clone()])
    }
}

#[derive(Clone, Default)]
struct CapturedOutput(Arc<StdMutex<Vec<u8>>>);
impl Write for CapturedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn native_calls_share_one_bounded_instance_and_preferences_remain_exclusive() {
    let state = Arc::new(NativeState::default());
    let entry = Arc::new(
        Source::from(Box::new(NativeExtension {
            state: state.clone(),
            preference: "initial".into(),
        }))
        .into_entry(2)
        .unwrap(),
    );
    let mut requests = Vec::new();
    for id in 1..=12 {
        let request = if id == 3 {
            WorkerRequest::SetPreferences {
                preferences: vec![Input::Text {
                    name: "token".into(),
                    state: Some("updated".into()),
                }],
            }
        } else {
            WorkerRequest::GetPages {
                path: if id <= 2 {
                    format!("overlap{id}")
                } else {
                    format!("page{id}")
                },
            }
        };
        write_frame_sync(&mut requests, &WorkerRequestEnvelope { id, request }).unwrap();
    }
    let captured = CapturedOutput::default();
    serve_requests(entry, Cursor::new(requests), captured.clone(), 2).unwrap();
    assert_eq!(state.peak.load(Ordering::SeqCst), 2);
    assert_eq!(state.active.load(Ordering::SeqCst), 0);
    assert!(!state.preference_overlap.load(Ordering::SeqCst));
    assert_eq!(state.preference_updates.load(Ordering::SeqCst), 1);
    let mut output = Cursor::new(captured.0.lock().unwrap().clone());
    assert!(matches!(
        read_frame_sync::<_, WorkerResponse>(&mut output).unwrap(),
        Some(WorkerResponse::Ready { .. })
    ));
    let mut replies = BTreeMap::new();
    while let Some(reply) = read_frame_sync::<_, WorkerResponse>(&mut output).unwrap() {
        let WorkerResponse::Result { id, value } = reply else {
            panic!("unexpected response: {reply:?}");
        };
        assert!(replies.insert(id, value).is_none());
    }
    assert_eq!(replies.len(), 12);
    assert!(matches!(replies.remove(&3), Some(WorkerValue::Unit)));
    for (id, value) in replies {
        let WorkerValue::Pages(pages) = value else {
            panic!("unexpected reply");
        };
        assert_eq!(
            pages[0],
            if id <= 2 {
                format!("overlap{id}")
            } else {
                format!("page{id}")
            }
        );
        if id <= 2 {
            assert_eq!(pages[1], "initial");
        }
    }
}
