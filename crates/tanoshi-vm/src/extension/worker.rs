use std::{
    collections::{BTreeMap, VecDeque},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::{Arc, Mutex as StdMutex},
    task::{Context as TaskContext, Poll},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::sync::atomic::{AtomicBool, Ordering};
use tanoshi_lib::prelude::{ChapterInfo, Input, MangaInfo, PluginDeclaration, SourceInfo};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader as AsyncBufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{Mutex, Notify, RwLock, RwLockReadGuard, RwLockWriteGuard, mpsc, oneshot, watch},
    task::JoinHandle,
    time::Instant,
};

use super::{
    Source, SourceEntry,
    source::{SourceAdmission, SourceHealth, panic_payload_message},
};

const PROTOCOL_VERSION: u32 = 2;
const MAX_FRAME_SIZE: usize = 128 * 1024 * 1024;
const INLINE_RESPONSE_LIMIT: usize = 64 * 1024;
const TIMEOUT_DRAIN_GRACE: Duration = Duration::from_secs(1);
const WORKER_BINARY_NAME: &str = "tanoshi-extension-worker";
pub const WORKER_MODE_FLAG: &str = "--tanoshi-extension-worker";

#[derive(Serialize, Deserialize)]
struct WorkerInitialization {
    protocol_version: u32,
    max_concurrent_calls: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum WorkerRequest {
    FilterList,
    GetPreferences,
    SetPreferences {
        preferences: Vec<Input>,
    },
    GetPopularManga {
        page: i64,
    },
    GetLatestManga {
        page: i64,
    },
    SearchManga {
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    },
    GetMangaDetail {
        path: String,
    },
    GetChapters {
        path: String,
    },
    GetPages {
        path: String,
    },
    GetImageBytes {
        url: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkerRequestEnvelope {
    id: u64,
    request: WorkerRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum WorkerResponse {
    Ready {
        protocol_version: u32,
        source_info: WorkerSourceInfo,
        rustc_version: String,
        lib_version: String,
    },
    Result {
        id: u64,
        value: WorkerValue,
    },
    Error {
        id: u64,
        kind: WorkerErrorKind,
        message: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) enum WorkerErrorKind {
    Operation,
    Panic,
    Protocol,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum WorkerValue {
    Unit,
    Inputs(Vec<Input>),
    MangaList(Vec<MangaInfo>),
    Manga(MangaInfo),
    Chapters(Vec<ChapterInfo>),
    Pages(Vec<String>),
    Image {
        #[serde(with = "base64_bytes")]
        bytes: Vec<u8>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WorkerSourceInfo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub version: String,
    pub icon: String,
    pub languages: tanoshi_lib::prelude::Lang,
    pub nsfw: bool,
}

impl From<&SourceInfo> for WorkerSourceInfo {
    fn from(source_info: &SourceInfo) -> Self {
        Self {
            id: source_info.id,
            name: source_info.name.clone(),
            url: source_info.url.clone(),
            version: source_info.version.to_string(),
            icon: source_info.icon.to_string(),
            languages: source_info.languages.clone(),
            nsfw: source_info.nsfw,
        }
    }
}

impl WorkerSourceInfo {
    pub(crate) fn into_source_info(self) -> SourceInfo {
        SourceInfo {
            id: self.id,
            name: self.name,
            url: self.url,
            version: Box::leak(self.version.into_boxed_str()),
            icon: Box::leak(self.icon.into_boxed_str()),
            languages: self.languages,
            nsfw: self.nsfw,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkerRestartReason {
    TimeoutRecovery,
    NotDispatched,
    Crash,
}

#[derive(Clone, Debug)]
pub(crate) enum WorkerCallError {
    /// The request exceeded its deadline. The worker gives its other calls a
    /// short grace period before termination; no further calls enter it.
    Timeout,
    /// The deadline expired before dispatch, including while waiting for a
    /// retiring worker to finish. Its running calls are left untouched.
    QueueTimeout,
    /// The client was explicitly shut down because its source was unloaded
    /// or replaced.
    Stopped,
    /// Health changed after admission, including during automatic retries.
    Admission(SourceAdmission),
    /// Another call or a transport failure caused the process to exit. This
    /// is not an additional source-health failure for each affected caller.
    Restarted {
        reason: WorkerRestartReason,
        message: String,
    },
    Crashed(String),
    Remote {
        kind: WorkerErrorKind,
        message: String,
    },
}

impl std::fmt::Display for WorkerCallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => formatter.write_str("extension worker request timed out"),
            Self::QueueTimeout => {
                formatter.write_str("extension worker is busy with earlier calls")
            }
            Self::Stopped => formatter.write_str("extension worker was shut down"),
            Self::Admission(admission) => {
                write!(formatter, "extension source is unavailable: {admission:?}")
            }
            Self::Restarted { reason, message } => write!(
                formatter,
                "extension worker restarted ({reason:?}): {message}"
            ),
            Self::Crashed(message) => write!(formatter, "extension worker exited: {message}"),
            Self::Remote { kind, message } => {
                write!(formatter, "extension worker returned {kind:?}: {message}")
            }
        }
    }
}

impl std::error::Error for WorkerCallError {}

type WorkerResult = std::result::Result<WorkerValue, WorkerCallError>;
type SavedPreferences = Arc<StdMutex<Option<Vec<Input>>>>;

#[derive(Debug)]
enum WorkerReply {
    Finished(WorkerResult),
    /// Calls still in the supervisor queue can return their original payload.
    Retry(WorkerRequest),
}

struct WorkerCall {
    request: WorkerRequest,
    deadline: Instant,
    reply: oneshot::Sender<WorkerReply>,
}

struct PendingCall {
    deadline: Instant,
    reply: Option<oneshot::Sender<WorkerReply>>,
    preferences: Option<Vec<Input>>,
    write_progress: Arc<StdMutex<WriteProgress>>,
}

#[derive(Default)]
struct WriteProgress {
    bytes_written: usize,
    complete: bool,
    cancelled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WritePhase {
    Unwritten,
    Partial,
    Complete,
}

impl WriteProgress {
    fn phase(&self) -> WritePhase {
        if self.complete {
            WritePhase::Complete
        } else if self.bytes_written == 0 {
            WritePhase::Unwritten
        } else {
            WritePhase::Partial
        }
    }
}

struct OutgoingCall {
    envelope: WorkerRequestEnvelope,
    deadline: Instant,
    progress: Arc<StdMutex<WriteProgress>>,
}

#[derive(Debug)]
enum WriteQueueError {
    Full,
    Closed,
}

#[derive(Default)]
struct WriteQueueState {
    calls: VecDeque<OutgoingCall>,
    closed: bool,
}

/// The supervisor can remove expired entries while the writer is blocked on
/// another frame. A bounded channel cannot release those occupied slots.
struct WriteQueue {
    state: StdMutex<WriteQueueState>,
    changed: Notify,
    capacity: usize,
}

impl WriteQueue {
    fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            state: StdMutex::new(WriteQueueState::default()),
            changed: Notify::new(),
            capacity,
        })
    }

    fn push(&self, call: OutgoingCall) -> std::result::Result<(), WriteQueueError> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed {
            return Err(WriteQueueError::Closed);
        }
        let now = Instant::now();
        state.calls.retain(|queued| {
            let mut progress = queued
                .progress
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if queued.deadline <= now {
                progress.cancelled = true;
            }
            !progress.cancelled
        });
        if state.calls.len() >= self.capacity {
            return Err(WriteQueueError::Full);
        }
        state.calls.push_back(call);
        drop(state);
        self.changed.notify_one();
        Ok(())
    }

    async fn next(&self) -> Option<OutgoingCall> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let mut state = self
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if state.closed {
                    return None;
                }
                if let Some(call) = state.calls.pop_front() {
                    return Some(call);
                }
            }
            changed.await;
        }
    }

    fn close(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.closed = true;
        state.calls.clear();
        drop(state);
        self.changed.notify_waiters();
    }
}

struct CloseWriteQueue(Arc<WriteQueue>);

impl Drop for CloseWriteQueue {
    fn drop(&mut self) {
        self.0.close();
    }
}

struct WorkerProcess {
    requests: mpsc::Sender<WorkerCall>,
    shutdown: watch::Sender<bool>,
    task: Option<JoinHandle<()>>,
    source_info: WorkerSourceInfo,
    rustc_version: String,
    lib_version: String,
}

impl WorkerProcess {
    async fn shutdown(&mut self) {
        self.shutdown.send_replace(true);
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for WorkerProcess {
    fn drop(&mut self) {
        self.shutdown.send_replace(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub(crate) struct WorkerClient {
    plugin_path: PathBuf,
    worker_path: PathBuf,
    startup_timeout: Duration,
    max_concurrent_calls: usize,
    cleanup_path: Option<PathBuf>,
    stopped: AtomicBool,
    // This lock protects process startup and replacement, not request execution.
    process: Mutex<Option<WorkerProcess>>,
    // Fair host-side admission keeps preference writers out of the native
    // RwLock until every read has completed, including response handling.
    preference_gate: RwLock<()>,
    shutdown: Notify,
    startup_preferences: SavedPreferences,
    pub(crate) health: Arc<SourceHealth>,
}

struct CallGateGuard<'a> {
    _read: Option<RwLockReadGuard<'a, ()>>,
    _write: Option<RwLockWriteGuard<'a, ()>>,
}

impl WorkerClient {
    fn ensure_available(&self) -> std::result::Result<(), WorkerCallError> {
        match self.health.admission() {
            SourceAdmission::Allowed => Ok(()),
            admission => Err(WorkerCallError::Admission(admission)),
        }
    }

    pub(crate) fn new(
        plugin_path: PathBuf,
        worker_path: PathBuf,
        startup_timeout: Duration,
        max_concurrent_calls: usize,
        cleanup_path: Option<PathBuf>,
    ) -> Arc<Self> {
        Arc::new(Self {
            plugin_path,
            worker_path,
            startup_timeout,
            max_concurrent_calls,
            cleanup_path,
            stopped: AtomicBool::new(false),
            process: Mutex::new(None),
            preference_gate: RwLock::new(()),
            shutdown: Notify::new(),
            startup_preferences: Arc::new(StdMutex::new(None)),
            health: SourceHealth::new(),
        })
    }

    pub(crate) async fn start(&self) -> Result<(WorkerSourceInfo, String, String)> {
        let mut process = self.process.lock().await;
        if self.stopped.load(Ordering::Acquire) {
            bail!("extension worker is shut down");
        }
        self.ensure_available()?;
        if process.is_none() {
            *process = Some(
                tokio::time::timeout(self.startup_timeout, self.spawn_process())
                    .await
                    .context("extension worker startup timed out")??,
            );
        }
        let process = process.as_ref().expect("worker process was initialized");
        Ok((
            process.source_info.clone(),
            process.rustc_version.clone(),
            process.lib_version.clone(),
        ))
    }

    pub(crate) async fn request(
        &self,
        mut request: WorkerRequest,
        timeout: Duration,
    ) -> WorkerResult {
        let deadline = Instant::now() + timeout;
        let mut retried_crash = false;
        let shutdown = self.shutdown.notified();
        tokio::pin!(shutdown);
        shutdown.as_mut().enable();
        if self.stopped.load(Ordering::Acquire) {
            return Err(WorkerCallError::Stopped);
        }
        self.ensure_available()?;
        let _gate = tokio::select! {
            guard = tokio::time::timeout_at(deadline, async {
                if matches!(&request, WorkerRequest::SetPreferences { .. }) {
                    CallGateGuard { _read: None, _write: Some(self.preference_gate.write().await) }
                } else {
                    CallGateGuard { _read: Some(self.preference_gate.read().await), _write: None }
                }
            }) => guard.map_err(|_| WorkerCallError::QueueTimeout)?,
            _ = &mut shutdown => return Err(WorkerCallError::Stopped),
        };

        loop {
            if self.stopped.load(Ordering::Acquire) {
                return Err(WorkerCallError::Stopped);
            }
            self.ensure_available()?;
            let mut process = tokio::select! {
                guard = tokio::time::timeout_at(deadline, self.process.lock()) =>
                    guard.map_err(|_| WorkerCallError::QueueTimeout)?,
                _ = &mut shutdown => return Err(WorkerCallError::Stopped),
            };
            if self.stopped.load(Ordering::Acquire) {
                return Err(WorkerCallError::Stopped);
            }
            self.ensure_available()?;

            if let Some(worker) = process.as_mut()
                && worker.requests.is_closed()
            {
                // Wait for the bounded drain and process termination before
                // another instance uses the source's HTTP/session state.
                if let Some(task) = worker.task.as_mut() {
                    tokio::select! {
                        result = tokio::time::timeout_at(deadline, task) => {
                            let _ = result.map_err(|_| WorkerCallError::QueueTimeout)?;
                        }
                        _ = &mut shutdown => return Err(WorkerCallError::Stopped),
                    }
                    worker.task.take();
                }
                process.take();
            }
            if process.is_none() {
                // The retiring process may have quarantined this source while
                // we waited for its task. Never spawn through that quarantine.
                self.ensure_available()?;
                let result = tokio::select! {
                    result = tokio::time::timeout_at(deadline, self.spawn_process()) => result,
                    _ = &mut shutdown => return Err(WorkerCallError::Stopped),
                };
                *process = Some(match result {
                    Ok(Ok(worker)) => worker,
                    Ok(Err(error)) => return Err(WorkerCallError::Crashed(error.to_string())),
                    Err(_) => return Err(WorkerCallError::QueueTimeout),
                });
            }
            if Instant::now() >= deadline {
                return Err(WorkerCallError::QueueTimeout);
            }
            self.ensure_available()?;
            let requests = process
                .as_ref()
                .expect("worker process was initialized")
                .requests
                .clone();
            drop(process);

            // Reads and setting the same preferences again are safe to retry
            // after an interrupted process, within the original time budget.
            let retry_request = request.clone();
            let (reply, response) = oneshot::channel();
            let call = WorkerCall {
                request,
                deadline,
                reply,
            };
            let sent = tokio::select! {
                sent = tokio::time::timeout_at(deadline, requests.send(call)) =>
                    sent.map_err(|_| WorkerCallError::QueueTimeout)?,
                _ = &mut shutdown => return Err(WorkerCallError::Stopped),
            };
            if let Err(error) = sent {
                // The process retired between cloning its channel and sending.
                // This request was never dispatched, so retry the replacement.
                request = error.0.request;
                continue;
            }
            match tokio::select! {
                result = response => result.unwrap_or_else(|_| WorkerReply::Finished(Err(WorkerCallError::Crashed(
                    "extension worker supervisor exited without a response".to_string(),
                )))),
                _ = &mut shutdown => WorkerReply::Finished(Err(WorkerCallError::Stopped)),
            } {
                WorkerReply::Finished(Err(WorkerCallError::Restarted { reason, message })) => {
                    self.ensure_available()?;
                    if Instant::now() >= deadline {
                        return Err(WorkerCallError::Timeout);
                    }
                    if matches!(reason, WorkerRestartReason::Crash) {
                        if retried_crash {
                            return Err(WorkerCallError::Restarted { reason, message });
                        }
                        // This budget belongs to the request, not source health:
                        // successful peers must not enable an endless crash loop.
                        retried_crash = true;
                    }
                    log::debug!("retrying interrupted extension call ({reason:?}): {message}");
                    request = retry_request;
                }
                WorkerReply::Finished(result) => return result,
                WorkerReply::Retry(unsent) => request = unsent,
            };
        }
    }

    pub(crate) async fn pause(&self) {
        self.stopped.store(true, Ordering::Release);
        self.shutdown.notify_waiters();
        let mut process = self.process.lock().await;
        if let Some(worker) = process.as_mut() {
            worker.shutdown().await;
        }
        process.take();
    }

    pub(crate) fn resume(&self) {
        self.stopped.store(false, Ordering::Release);
    }

    pub(crate) async fn shutdown(&self) {
        self.pause().await;
        self.cleanup_path();
    }

    fn cleanup_path(&self) {
        let Some(path) = self.cleanup_path.as_deref() else {
            return;
        };
        if let Err(error) = std::fs::remove_file(path)
            && error.kind() != io::ErrorKind::NotFound
        {
            log::warn!(
                "failed to remove extension worker staging file {}: {error}; retrying at startup",
                path.display()
            );
        }
    }

    async fn spawn_process(&self) -> Result<WorkerProcess> {
        let mut child = Command::new(&self.worker_path)
            .arg(WORKER_MODE_FLAG)
            .arg("--plugin")
            .arg(&self.plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| {
                format!(
                    "failed to start extension worker {} for {}",
                    self.worker_path.display(),
                    self.plugin_path.display()
                )
            })?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("extension worker stdin was not piped"))?;
        let mut stdout = AsyncBufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| anyhow!("extension worker stdout was not piped"))?,
        );
        write_frame_async(
            &mut stdin,
            &WorkerInitialization {
                protocol_version: PROTOCOL_VERSION,
                max_concurrent_calls: self.max_concurrent_calls,
            },
        )
        .await?;
        let response = read_frame_async::<_, WorkerResponse>(&mut stdout)
            .await
            .context("failed to read extension worker readiness")?;
        let (source_info, rustc_version, lib_version) = match response {
            WorkerResponse::Ready {
                protocol_version,
                source_info,
                rustc_version,
                lib_version,
            } if protocol_version == PROTOCOL_VERSION => (source_info, rustc_version, lib_version),
            WorkerResponse::Ready {
                protocol_version, ..
            } => {
                bail!(
                    "extension worker protocol mismatch: worker={protocol_version} host={PROTOCOL_VERSION}"
                );
            }
            other => bail!("extension worker did not send readiness: {other:?}"),
        };
        let next_request_id = self
            .apply_startup_preferences(&mut stdin, &mut stdout)
            .await?;
        let (requests, incoming) = mpsc::channel(self.max_concurrent_calls);
        let (shutdown, stopped) = watch::channel(false);
        let preferences = self.startup_preferences.clone();
        let health = self.health.clone();
        let max_concurrent_calls = self.max_concurrent_calls;
        let task = tokio::spawn(async move {
            dispatch_requests(
                stdin,
                stdout,
                DispatcherState {
                    requests: incoming,
                    shutdown: stopped,
                    preferences,
                    health,
                    next_request_id,
                    max_concurrent_calls,
                },
            )
            .await;
            terminate_process(&mut child).await;
        });
        Ok(WorkerProcess {
            requests,
            shutdown,
            task: Some(task),
            source_info,
            rustc_version,
            lib_version,
        })
    }

    /// Replay preferences before making the process available to callers.
    async fn apply_startup_preferences(
        &self,
        stdin: &mut ChildStdin,
        stdout: &mut AsyncBufReader<ChildStdout>,
    ) -> Result<u64> {
        let preferences = self
            .startup_preferences
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let Some(preferences) = preferences else {
            return Ok(1);
        };
        let envelope = WorkerRequestEnvelope {
            id: 1,
            request: WorkerRequest::SetPreferences { preferences },
        };
        write_frame_async(stdin, &envelope).await?;
        let response = read_frame_async::<_, WorkerResponse>(stdout)
            .await
            .context("failed to apply saved preferences to the extension worker")?;
        match response {
            WorkerResponse::Result {
                id: 1,
                value: WorkerValue::Unit,
            } => Ok(2),
            WorkerResponse::Error { kind, message, .. } => {
                bail!("extension worker rejected saved preferences ({kind:?}): {message}")
            }
            other => bail!(
                "extension worker sent an unexpected response to saved preferences: {other:?}"
            ),
        }
    }
}

impl Drop for WorkerClient {
    fn drop(&mut self) {
        if let Ok(mut process) = self.process.try_lock() {
            process.take();
        }
        self.cleanup_path();
    }
}

enum TransportEvent {
    Response(WorkerResponse),
    Failed { message: String },
    WriteExpired { id: u64, phase: WritePhase },
    Rejected { id: u64, admission: SourceAdmission },
}

struct TransportTasks {
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}

impl Drop for TransportTasks {
    fn drop(&mut self) {
        self.reader.abort();
        self.writer.abort();
    }
}

struct DispatcherState {
    requests: mpsc::Receiver<WorkerCall>,
    shutdown: watch::Receiver<bool>,
    preferences: SavedPreferences,
    health: Arc<SourceHealth>,
    next_request_id: u64,
    max_concurrent_calls: usize,
}

async fn dispatch_requests<R, W>(stdin: W, stdout: R, state: DispatcherState)
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let DispatcherState {
        mut requests,
        mut shutdown,
        preferences,
        health,
        mut next_request_id,
        max_concurrent_calls,
    } = state;
    let outgoing = WriteQueue::new(max_concurrent_calls);
    let (events_tx, mut events) = mpsc::channel(max_concurrent_calls);
    let _transport = TransportTasks {
        reader: tokio::spawn(read_responses(stdout, events_tx.clone())),
        writer: tokio::spawn(write_requests(
            stdin,
            outgoing.clone(),
            events_tx,
            health.clone(),
        )),
    };
    let mut pending = BTreeMap::<u64, PendingCall>::new();
    let mut retirement_deadline = None;
    let failure = loop {
        if *shutdown.borrow() {
            break Some(WorkerCallError::Stopped);
        }
        if retirement_deadline.is_some() && pending.values().all(|call| call.reply.is_none()) {
            break None;
        }
        let next_deadline = pending
            .values()
            .filter(|call| call.reply.is_some())
            .map(|call| call.deadline)
            .chain(retirement_deadline)
            .min();
        tokio::select! {
            biased;
            _ = shutdown.changed() => break Some(WorkerCallError::Stopped),
            _ = wait_until(next_deadline) => {
                let now = Instant::now();
                if retirement_deadline.is_some_and(|deadline| deadline <= now) {
                    break Some(WorkerCallError::Restarted {
                        reason: WorkerRestartReason::TimeoutRecovery,
                        message: "the timeout recovery grace period elapsed".to_string(),
                    });
                }
                let mut expired_unwritten = Vec::new();
                let mut executing_timeout = false;
                let mut partial_timeout = false;
                for (id, call) in pending.iter_mut().filter(|(_, call)| call.reply.is_some() && call.deadline <= now) {
                    // Hold the same lock that covers poll_write's deadline
                    // check and first bytes. Cancellation cannot race a write
                    // that still appears to be queued.
                    let mut progress = call.write_progress.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    let error = if progress.complete {
                        executing_timeout = true;
                        WorkerCallError::Timeout
                    } else {
                        progress.cancelled = true;
                        if progress.bytes_written == 0 {
                            expired_unwritten.push(*id);
                            WorkerCallError::QueueTimeout
                        } else {
                            partial_timeout = true;
                            WorkerCallError::Timeout
                        }
                    };
                    if let Some(reply) = call.reply.take() {
                        let _ = reply.send(WorkerReply::Finished(Err(error)));
                    }
                }
                for id in expired_unwritten {
                    pending.remove(&id);
                }
                if partial_timeout {
                    // A truncated frame cannot share its stream with later
                    // requests. Stop the writer and process without draining.
                    break Some(WorkerCallError::Restarted {
                        reason: WorkerRestartReason::TimeoutRecovery,
                        message: "a request timed out during a partial write".to_string(),
                    });
                }
                if executing_timeout {
                    retirement_deadline.get_or_insert(now + TIMEOUT_DRAIN_GRACE);
                    requests.close();
                    while let Ok(call) = requests.try_recv() {
                        let _ = call.reply.send(WorkerReply::Retry(call.request));
                    }
                }
            },
            event = events.recv() => match event {
                Some(TransportEvent::Response(response)) => {
                    let (id, result) = match response {
                        WorkerResponse::Result { id, value } => (id, Ok(value)),
                        WorkerResponse::Error { id, kind, message } =>
                            (id, Err(WorkerCallError::Remote { kind, message })),
                        WorkerResponse::Ready { .. } => break Some(WorkerCallError::Crashed(
                            "worker sent an unexpected readiness response".to_string(),
                        )),
                    };
                    let Some(mut call) = pending.remove(&id) else {
                        break Some(WorkerCallError::Crashed(format!(
                            "worker sent a response for unknown request id {id}",
                        )));
                    };
                    if let Some(reply) = call.reply.take() {
                        if matches!(&result, Ok(WorkerValue::Unit))
                            && let Some(updated) = call.preferences
                        {
                            // Record an acknowledged change before the reply
                            // reaches its caller or a replacement process starts.
                            *preferences.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(updated);
                        }
                        let _ = reply.send(WorkerReply::Finished(result));
                    }
                }
                Some(TransportEvent::Failed { message }) =>
                    break Some(WorkerCallError::Crashed(message)),
                Some(TransportEvent::WriteExpired { id, phase }) => {
                    if phase == WritePhase::Complete {
                        // Keep the id to recognize a late reply, and give
                        // already executing peers the usual drain period.
                        if let Some(call) = pending.get_mut(&id) {
                            if let Some(reply) = call.reply.take() {
                                let _ = reply.send(WorkerReply::Finished(Err(WorkerCallError::Timeout)));
                            }
                            retirement_deadline.get_or_insert(Instant::now() + TIMEOUT_DRAIN_GRACE);
                            requests.close();
                            while let Ok(call) = requests.try_recv() {
                                let _ = call.reply.send(WorkerReply::Retry(call.request));
                            }
                        }
                    } else {
                        if let Some(mut call) = pending.remove(&id)
                            && let Some(reply) = call.reply.take()
                        {
                            let error = if phase == WritePhase::Partial { WorkerCallError::Timeout } else { WorkerCallError::QueueTimeout };
                            let _ = reply.send(WorkerReply::Finished(Err(error)));
                        }
                        if phase == WritePhase::Partial {
                            break Some(WorkerCallError::Restarted {
                                reason: WorkerRestartReason::TimeoutRecovery,
                                message: "a request timed out during a partial write".to_string(),
                            });
                        }
                    }
                }
                Some(TransportEvent::Rejected { id, admission }) => {
                    if let Some(mut call) = pending.remove(&id)
                        && let Some(reply) = call.reply.take()
                    {
                        let _ = reply.send(WorkerReply::Finished(Err(WorkerCallError::Admission(admission))));
                    }
                }
                None => break Some(WorkerCallError::Crashed(
                    "extension worker transport stopped".to_string(),
                )),
            },
            call = requests.recv(), if retirement_deadline.is_none() => match call {
                Some(call) if call.deadline <= Instant::now() => {
                    let _ = call.reply.send(WorkerReply::Finished(Err(WorkerCallError::QueueTimeout)));
                }
                Some(call) => {
                    let admission = health.admission();
                    if admission != SourceAdmission::Allowed {
                        let _ = call.reply.send(WorkerReply::Finished(Err(WorkerCallError::Admission(admission))));
                        continue;
                    }
                    let id = next_request_id;
                    next_request_id = next_request_id.wrapping_add(1);
                    let updated = match &call.request {
                        WorkerRequest::SetPreferences { preferences } => Some(preferences.clone()),
                        _ => None,
                    };
                    let progress = Arc::new(StdMutex::new(WriteProgress::default()));
                    pending.insert(id, PendingCall {
                        deadline: call.deadline,
                        reply: Some(call.reply),
                        preferences: updated,
                        write_progress: progress.clone(),
                    });
                    match outgoing.push(OutgoingCall {
                        envelope: WorkerRequestEnvelope { id, request: call.request },
                        deadline: call.deadline,
                        progress,
                    }) {
                        Ok(()) => {},
                        Err(WriteQueueError::Full) => {
                            // Capacity pressure is an admission failure, never
                            // evidence that the extension process crashed.
                            if let Some(mut call) = pending.remove(&id)
                                && let Some(reply) = call.reply.take()
                            {
                                let _ = reply.send(WorkerReply::Finished(Err(WorkerCallError::QueueTimeout)));
                            }
                        }
                        Err(WriteQueueError::Closed) => break Some(WorkerCallError::Crashed(
                            "extension worker request writer stopped".to_string(),
                        )),
                    }
                }
                None => break Some(WorkerCallError::Stopped),
            },
        }
    };
    requests.close();
    outgoing.close();
    if let Some(error) = failure {
        // EOF, a broken pipe, or a protocol failure cannot identify which
        // concurrently executing extension call caused the process to fail.
        // Count the event once and do not blame a preference writer by age.
        if matches!(error, WorkerCallError::Crashed(_)) {
            if pending.is_empty() {
                log::warn!("idle extension worker exited without an active call: {error}");
            } else {
                let quarantined = health.record_failure();
                log::error!("EXTENSION WORKER CRASH: {error}; quarantined={quarantined}");
            }
        }
        for (_, mut call) in pending {
            if let Some(reply) = call.reply.take() {
                let error = if matches!(error, WorkerCallError::Crashed(_)) {
                    let mut progress = call
                        .write_progress
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    let reason = if progress.bytes_written == 0 {
                        // Prevent a first byte from racing this classification.
                        progress.cancelled = true;
                        WorkerRestartReason::NotDispatched
                    } else {
                        WorkerRestartReason::Crash
                    };
                    WorkerCallError::Restarted {
                        reason,
                        message: error.to_string(),
                    }
                } else {
                    error.clone()
                };
                let _ = reply.send(WorkerReply::Finished(Err(error)));
            }
        }
    }
    // These calls never reached the writer. Retry them on the replacement
    // under their original deadlines, including calls already in this queue.
    // A sender that reserved capacity before close can still enqueue a call.
    // Receive until the closed channel is fully drained, including reservations.
    while let Some(call) = requests.recv().await {
        let _ = call.reply.send(WorkerReply::Retry(call.request));
    }
}

async fn wait_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

async fn read_responses<R>(mut stdout: R, events: mpsc::Sender<TransportEvent>)
where
    R: AsyncRead + Unpin,
{
    loop {
        let response = match read_frame_async_bytes(&mut stdout).await {
            Ok(bytes) if bytes.len() <= INLINE_RESPONSE_LIMIT => {
                serde_json::from_slice::<WorkerResponse>(&bytes).map_err(io::Error::other)
            }
            Ok(bytes) => tokio::task::spawn_blocking(move || {
                serde_json::from_slice::<WorkerResponse>(&bytes)
            })
            .await
            .map_err(io::Error::other)
            .and_then(|result| result.map_err(io::Error::other)),
            Err(error) => Err(error),
        };
        let event = match response {
            Ok(response) => TransportEvent::Response(response),
            Err(error) => TransportEvent::Failed {
                message: error.to_string(),
            },
        };
        let failed = matches!(event, TransportEvent::Failed { .. });
        if events.send(event).await.is_err() || failed {
            break;
        }
    }
}

async fn write_requests<W>(
    mut stdin: W,
    requests: Arc<WriteQueue>,
    events: mpsc::Sender<TransportEvent>,
    health: Arc<SourceHealth>,
) where
    W: AsyncWrite + Unpin,
{
    let _close = CloseWriteQueue(requests.clone());
    while let Some(request) = requests.next().await {
        let id = request.envelope.id;
        let result = match serialize_frame(&request.envelope).map_err(io::Error::other) {
            Ok(frame) => {
                let mut writer = DeadlineWriter {
                    writer: &mut stdin,
                    progress: &request.progress,
                    deadline: request.deadline,
                    health: &health,
                    frame_len: frame.len(),
                };
                tokio::time::timeout_at(request.deadline, async {
                    writer.write_all(&frame).await?;
                    writer.flush().await
                })
                .await
                .unwrap_or_else(|_| {
                    Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "request write deadline expired",
                    ))
                })
            }
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            let phase = request
                .progress
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .phase();
            let admission = health.admission();
            let (event, stop) = if error.kind() == io::ErrorKind::TimedOut {
                (
                    TransportEvent::WriteExpired { id, phase },
                    phase == WritePhase::Partial,
                )
            } else if phase == WritePhase::Unwritten && admission != SourceAdmission::Allowed {
                (TransportEvent::Rejected { id, admission }, false)
            } else {
                (
                    TransportEvent::Failed {
                        message: error.to_string(),
                    },
                    true,
                )
            };
            if events.send(event).await.is_err() || stop {
                break;
            }
        }
    }
}

/// A write that is still waiting for its first byte can be cancelled without
/// retiring the process. Lock progress across poll_write so the supervisor's
/// cancellation and the first bytes have a single ordering, even across threads.
struct DeadlineWriter<'a, W> {
    writer: &'a mut W,
    progress: &'a StdMutex<WriteProgress>,
    deadline: Instant,
    health: &'a SourceHealth,
    frame_len: usize,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for DeadlineWriter<'_, W> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut TaskContext<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let mut progress = this
            .progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if progress.cancelled || Instant::now() >= this.deadline {
            progress.cancelled = true;
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "request write deadline expired",
            )));
        }
        if progress.bytes_written == 0 && this.health.admission() != SourceAdmission::Allowed {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "source admission was revoked",
            )));
        }
        let result = Pin::new(&mut *this.writer).poll_write(context, bytes);
        if let Poll::Ready(Ok(written)) = &result {
            progress.bytes_written += written;
            // All frame bytes were accepted by ChildStdin: written to the pipe
            // on Unix, or buffered for a blocking write on Windows. Keep the
            // drain period even if flush misses the deadline.
            progress.complete = progress.bytes_written == this.frame_len;
        }
        result
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let mut progress = this
            .progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if progress.cancelled || Instant::now() >= this.deadline {
            progress.cancelled = true;
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "request write deadline expired",
            )));
        }
        Pin::new(&mut *this.writer).poll_flush(context)
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.get_mut().writer).poll_shutdown(context)
    }
}

pub(crate) fn resolve_worker_path() -> PathBuf {
    if let Some(path) = std::env::var_os("TANOSHI_EXTENSION_WORKER") {
        return PathBuf::from(path);
    }

    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        let worker_name = if cfg!(windows) {
            format!("{WORKER_BINARY_NAME}.exe")
        } else {
            WORKER_BINARY_NAME.to_string()
        };
        let sibling = parent.join(worker_name);
        if sibling.is_file() {
            return sibling;
        }
    }

    PathBuf::from(WORKER_BINARY_NAME)
}

pub fn run_worker(plugin_path: PathBuf) -> Result<()> {
    // The host binaries skip their own logger setup in worker mode, so give
    // worker-side log output a stderr logger of its own; stderr is inherited
    // by the host process.
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    let mut input = BufReader::new(io::stdin().lock());
    let initialization = read_frame_sync::<_, WorkerInitialization>(&mut input)?
        .ok_or_else(|| anyhow!("missing extension worker initialization"))?;
    if initialization.protocol_version != PROTOCOL_VERSION {
        bail!(
            "extension worker protocol mismatch: host={} worker={PROTOCOL_VERSION}",
            initialization.protocol_version
        );
    }
    if initialization.max_concurrent_calls == 0
        || initialization.max_concurrent_calls > tokio::sync::Semaphore::MAX_PERMITS
    {
        bail!("invalid extension worker concurrency");
    }
    let entry = load_worker_entry(&plugin_path, initialization.max_concurrent_calls)?;
    serve_requests(
        entry,
        input,
        BufWriter::new(io::stdout()),
        initialization.max_concurrent_calls,
    )
}

fn serve_requests<R, W>(
    entry: Arc<SourceEntry>,
    mut input: R,
    mut output: W,
    max_concurrent_calls: usize,
) -> Result<()>
where
    R: Read,
    W: Write + Send + 'static,
{
    write_frame_sync(
        &mut output,
        &WorkerResponse::Ready {
            protocol_version: PROTOCOL_VERSION,
            source_info: WorkerSourceInfo::from(&entry.source_info),
            rustc_version: entry.rustc_version.clone(),
            lib_version: entry.lib_version.clone(),
        },
    )?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(max_concurrent_calls)
        .thread_name("tanoshi-extension")
        .build()?;
    let output = Arc::new(StdMutex::new(output));
    while let Some(request) = read_frame_sync::<_, WorkerRequestEnvelope>(&mut input)? {
        let permit = runtime.block_on(entry.limiter.clone().acquire_owned())?;
        let entry = entry.clone();
        let output = output.clone();
        runtime.spawn_blocking(move || {
            if let Err(error) = catch_worker_job(move || {
                // Keep the permit through serialization and output to bound
                // completed image buffers even when the pipe is slow.
                let _permit = permit;
                write_response(&output, execute_envelope(&entry, request))
            }) {
                log::error!("extension worker response task failed: {error}");
                // spawn_blocking otherwise swallows panics. A broken or
                // missing response must make the host restart immediately.
                std::process::exit(1);
            }
        });
    }

    Ok(())
}

fn catch_worker_job(job: impl FnOnce() -> Result<()>) -> Result<()> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
        .map_err(|panic| anyhow!("response task panicked: {}", panic_payload_message(&*panic)))?
}

fn write_response<W: Write>(output: &StdMutex<W>, response: WorkerResponse) -> Result<()> {
    let frame = match serialize_frame(&response) {
        Ok(frame) => Ok(frame),
        Err(error) => {
            let id = match response {
                WorkerResponse::Result { id, .. } | WorkerResponse::Error { id, .. } => id,
                WorkerResponse::Ready { .. } => unreachable!(),
            };
            serialize_frame(&WorkerResponse::Error {
                id,
                kind: WorkerErrorKind::Protocol,
                message: error.to_string(),
            })
        }
    }?;
    // Release image buffers before waiting for other responses to write.
    drop(response);
    let mut output = output
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    output.write_all(&frame)?;
    output.flush()?;
    Ok(())
}

fn execute_envelope(entry: &Arc<SourceEntry>, request: WorkerRequestEnvelope) -> WorkerResponse {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_request(entry, request.request)
    })) {
        Ok(Ok(value)) => WorkerResponse::Result {
            id: request.id,
            value,
        },
        Ok(Err(error)) => WorkerResponse::Error {
            id: request.id,
            kind: WorkerErrorKind::Operation,
            message: error.to_string(),
        },
        Err(payload) => WorkerResponse::Error {
            id: request.id,
            kind: WorkerErrorKind::Panic,
            message: panic_payload_message(&*payload),
        },
    }
}

fn load_worker_entry(plugin_path: &Path, max_concurrent_calls: usize) -> Result<Arc<SourceEntry>> {
    let library = unsafe { libloading::Library::new(plugin_path) }?;
    let declaration = unsafe {
        library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read()
    };
    if declaration.rustc_version != tanoshi_lib::RUSTC_VERSION {
        bail!(
            "Version mismatch: extension.rustc_version={} != tanoshi_lib.rustc_version={}",
            declaration.rustc_version,
            tanoshi_lib::RUSTC_VERSION,
        );
    }
    if declaration.core_version != tanoshi_lib::LIB_VERSION {
        bail!(
            "Version mismatch: extension.lib_version={} != tanoshi_lib.lib_version={}",
            declaration.core_version,
            tanoshi_lib::LIB_VERSION
        );
    }

    let mut source = Source::new(library, declaration.rustc_version, declaration.core_version)
        .with_plugin_path(plugin_path.to_path_buf());
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        (declaration.register)(&mut source);
    }))
    .map_err(|payload| {
        anyhow!(
            "extension registration panicked: {}",
            panic_payload_message(&*payload)
        )
    })?;

    Ok(Arc::new(source.into_entry(max_concurrent_calls)?))
}

fn execute_request(entry: &Arc<SourceEntry>, request: WorkerRequest) -> Result<WorkerValue> {
    match request {
        WorkerRequest::FilterList => {
            entry.with_extension(|extension| Ok(WorkerValue::Inputs(extension.filter_list())))
        }
        WorkerRequest::GetPreferences => entry
            .with_extension(|extension| extension.get_preferences())
            .map(WorkerValue::Inputs),
        WorkerRequest::SetPreferences { preferences } => entry
            .with_extension_mut(|extension| extension.set_preferences(preferences))
            .map(|()| WorkerValue::Unit),
        WorkerRequest::GetPopularManga { page } => entry
            .with_extension(|extension| extension.get_popular_manga(page))
            .map(WorkerValue::MangaList),
        WorkerRequest::GetLatestManga { page } => entry
            .with_extension(|extension| extension.get_latest_manga(page))
            .map(WorkerValue::MangaList),
        WorkerRequest::SearchManga {
            page,
            query,
            filters,
        } => entry
            .with_extension(|extension| extension.search_manga(page, query, filters))
            .map(WorkerValue::MangaList),
        WorkerRequest::GetMangaDetail { path } => entry
            .with_extension(|extension| extension.get_manga_detail(path))
            .map(WorkerValue::Manga),
        WorkerRequest::GetChapters { path } => entry
            .with_extension(|extension| extension.get_chapters(path))
            .map(WorkerValue::Chapters),
        WorkerRequest::GetPages { path } => entry
            .with_extension(|extension| extension.get_pages(path))
            .map(WorkerValue::Pages),
        WorkerRequest::GetImageBytes { url } => entry
            .with_extension(|extension| extension.get_image_bytes(url))
            .map(|bytes| WorkerValue::Image {
                bytes: bytes.to_vec(),
            }),
    }
}

async fn terminate_process(child: &mut Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

async fn write_frame_async<W, T>(writer: &mut W, value: &T) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let frame = serialize_frame(value).map_err(io::Error::other)?;
    writer.write_all(&frame).await?;
    writer.flush().await
}

async fn read_frame_async<R, T>(reader: &mut R) -> io::Result<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let bytes = read_frame_async_bytes(reader).await?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

async fn read_frame_async_bytes<R>(reader: &mut R) -> io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut length = [0; 4];
    reader.read_exact(&mut length).await?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("worker frame exceeds {MAX_FRAME_SIZE} bytes"),
        ));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).await?;
    Ok(bytes)
}

fn write_frame_sync<W, T>(writer: &mut W, value: &T) -> io::Result<()>
where
    W: Write,
    T: Serialize,
{
    writer.write_all(&serialize_frame(value).map_err(io::Error::other)?)?;
    writer.flush()
}

fn read_frame_sync<R, T>(reader: &mut R) -> io::Result<Option<T>>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut length = [0; 4];
    let first = reader.read(&mut length[..1])?;
    if first == 0 {
        return Ok(None);
    }
    reader.read_exact(&mut length[1..])?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("worker frame exceeds {MAX_FRAME_SIZE} bytes"),
        ));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(io::Error::other)
}

fn serialize_frame<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(value)?;
    let length = u32::try_from(payload.len()).context("worker frame is too large")?;
    if payload.len() > MAX_FRAME_SIZE {
        bail!("worker frame exceeds {MAX_FRAME_SIZE} bytes");
    }
    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

mod base64_bytes {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests;
