use std::{
    future::Future,
    panic::AssertUnwindSafe,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex as StdMutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use fnv::FnvHashMap;
use tanoshi_lib::prelude::{ChapterInfo, Input, Lang, MangaInfo, SourceInfo};
use tokio::sync::{Mutex as AsyncMutex, OwnedSemaphorePermit};

use crate::{
    PLUGIN_EXTENSION,
    prelude::{Source, SourceEntry},
};

use super::source::{
    SOURCE_MAX_ABANDONED_CALLS, SourceAdmission, SourceHealth, panic_payload_message,
};
use super::worker::{
    WorkerCallError, WorkerClient, WorkerErrorKind, WorkerRequest, WorkerValue, resolve_worker_path,
};

const STAGED_LIBRARY_PREFIX: &str = ".tanoshi-staged-";
const INSTALL_TEMP_PREFIX: &str = ".tanoshi-install-";
const INSTALL_BACKUP_PREFIX: &str = ".tanoshi-backup-";
const CALL_RUNNING: u8 = 0;
const CALL_ABANDONED: u8 = 1;
const CALL_COMPLETE: u8 = 2;
static UNIQUE_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);
pub const DEFAULT_MAX_CONCURRENT_CALLS: usize = 8;
pub const DEFAULT_ADMISSION_TIMEOUT: Duration = Duration::from_secs(1);
pub const DEFAULT_METADATA_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_IMAGE_TIMEOUT: Duration = Duration::from_secs(120);
const MIN_EXTENSION_TIMEOUT: Duration = Duration::from_millis(1);

struct AbandonedCallTracker {
    state: AtomicU8,
    health: Arc<SourceHealth>,
}

impl AbandonedCallTracker {
    fn new(health: Arc<SourceHealth>) -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(CALL_RUNNING),
            health,
        })
    }

    fn mark_abandoned(&self) -> Option<usize> {
        self.state
            .compare_exchange(
                CALL_RUNNING,
                CALL_ABANDONED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .ok()
            .map(|_| self.health.start_abandoned_call())
    }

    fn complete(&self) {
        if self
            .state
            .compare_exchange(
                CALL_RUNNING,
                CALL_COMPLETE,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            return;
        }

        if self
            .state
            .compare_exchange(
                CALL_ABANDONED,
                CALL_COMPLETE,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            self.health.complete_abandoned_call();
        }
    }
}

fn is_managed_library_name(name: &str) -> bool {
    name.starts_with(STAGED_LIBRARY_PREFIX)
        || name.starts_with(INSTALL_TEMP_PREFIX)
        || name.starts_with(INSTALL_BACKUP_PREFIX)
}

fn normalize_plugin_name(name: &str) -> String {
    let name = Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(name);
    let extension = format!(".{PLUGIN_EXTENSION}");
    name.strip_suffix(&extension).unwrap_or(name).to_lowercase()
}

fn entry_plugin_name(entry: &SourceEntry) -> String {
    entry
        .plugin_path()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(normalize_plugin_name)
        .unwrap_or_else(|| normalize_plugin_name(entry.source_name()))
}

fn unexpected_worker_value(expected: &str, value: WorkerValue) -> anyhow::Error {
    anyhow!("expected {expected} response, got {value:?}")
}

fn decode_unit(value: WorkerValue) -> Result<()> {
    match value {
        WorkerValue::Unit => Ok(()),
        value => Err(unexpected_worker_value("unit", value)),
    }
}

fn decode_inputs(value: WorkerValue) -> Result<Vec<Input>> {
    match value {
        WorkerValue::Inputs(value) => Ok(value),
        value => Err(unexpected_worker_value("inputs", value)),
    }
}

fn decode_manga_list(value: WorkerValue) -> Result<Vec<MangaInfo>> {
    match value {
        WorkerValue::MangaList(value) => Ok(value),
        value => Err(unexpected_worker_value("manga list", value)),
    }
}

fn decode_manga(value: WorkerValue) -> Result<MangaInfo> {
    match value {
        WorkerValue::Manga(value) => Ok(value),
        value => Err(unexpected_worker_value("manga", value)),
    }
}

fn decode_chapters(value: WorkerValue) -> Result<Vec<ChapterInfo>> {
    match value {
        WorkerValue::Chapters(value) => Ok(value),
        value => Err(unexpected_worker_value("chapters", value)),
    }
}

fn decode_pages(value: WorkerValue) -> Result<Vec<String>> {
    match value {
        WorkerValue::Pages(value) => Ok(value),
        value => Err(unexpected_worker_value("pages", value)),
    }
}

fn decode_image(value: WorkerValue) -> Result<Bytes> {
    match value {
        WorkerValue::Image { bytes } => Ok(Bytes::from(bytes)),
        value => Err(unexpected_worker_value("image", value)),
    }
}

fn cleanup_managed_libraries(dir: &Path) {
    let current_process_marker = format!("-{}-", std::process::id());
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            warn!(
                "failed to scan {} for stale extension files: {error}",
                dir.display()
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !is_managed_library_name(name) {
            continue;
        }
        if name.contains(&current_process_marker) {
            continue;
        }

        if let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                "failed to remove stale extension file {}: {error}",
                path.display()
            );
        }
    }
}

struct PluginFileReplacement {
    backup_path: Option<PathBuf>,
}

#[derive(Clone, Copy)]
struct ExtensionCall {
    operation: &'static str,
    timeout: Duration,
    quarantine_on_panic: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ExtensionManagerOptions {
    pub max_concurrent_calls: usize,
    pub admission_timeout: Duration,
    pub metadata_timeout: Duration,
    pub image_timeout: Duration,
}

impl Default for ExtensionManagerOptions {
    fn default() -> Self {
        Self {
            max_concurrent_calls: DEFAULT_MAX_CONCURRENT_CALLS,
            admission_timeout: DEFAULT_ADMISSION_TIMEOUT,
            metadata_timeout: DEFAULT_METADATA_TIMEOUT,
            image_timeout: DEFAULT_IMAGE_TIMEOUT,
        }
    }
}

#[derive(Clone)]
pub struct ExtensionManager {
    dir: PathBuf,
    extensions: Arc<RwLock<FnvHashMap<i64, Arc<SourceEntry>>>>,
    lifecycle_locks: Arc<StdMutex<FnvHashMap<String, Arc<AsyncMutex<()>>>>>,
    options: ExtensionManagerOptions,
    worker_path: PathBuf,
}

pub fn dummy_source_info(id: i64) -> SourceInfo {
    SourceInfo {
        id,
        name: format!("Missing source {id}"),
        url: String::new(),
        version: "",
        icon: "",
        languages: Lang::All,
        nsfw: false,
    }
}

impl ExtensionManager {
    pub fn new<P: AsRef<Path>>(extension_dir: P) -> Self {
        Self::new_with_options(extension_dir, ExtensionManagerOptions::default())
    }

    pub fn new_with_options<P: AsRef<Path>>(
        extension_dir: P,
        mut options: ExtensionManagerOptions,
    ) -> Self {
        let configured_max_concurrent_calls = options.max_concurrent_calls;
        options.max_concurrent_calls =
            configured_max_concurrent_calls.clamp(1, tokio::sync::Semaphore::MAX_PERMITS);
        if configured_max_concurrent_calls == 0 {
            warn!("configured extension concurrency is zero; using one call per source instead");
        } else if configured_max_concurrent_calls > tokio::sync::Semaphore::MAX_PERMITS {
            warn!(
                "configured extension concurrency {} exceeds Tokio's maximum {}; using {} instead",
                configured_max_concurrent_calls,
                tokio::sync::Semaphore::MAX_PERMITS,
                tokio::sync::Semaphore::MAX_PERMITS
            );
        }
        if options.admission_timeout.is_zero() {
            warn!(
                "configured extension admission timeout is zero; using {:?} instead",
                MIN_EXTENSION_TIMEOUT
            );
            options.admission_timeout = MIN_EXTENSION_TIMEOUT;
        }
        if options.metadata_timeout.is_zero() {
            warn!(
                "configured extension metadata timeout is zero; using {:?} instead",
                MIN_EXTENSION_TIMEOUT
            );
            options.metadata_timeout = MIN_EXTENSION_TIMEOUT;
        }
        if options.image_timeout.is_zero() {
            warn!(
                "configured extension image timeout is zero; using {:?} instead",
                MIN_EXTENSION_TIMEOUT
            );
            options.image_timeout = MIN_EXTENSION_TIMEOUT;
        }
        let dir = PathBuf::new().join(extension_dir);
        cleanup_managed_libraries(&dir);
        Self {
            dir,
            extensions: Arc::new(RwLock::new(FnvHashMap::default())),
            lifecycle_locks: Arc::new(StdMutex::new(FnvHashMap::default())),
            options,
            worker_path: resolve_worker_path(),
        }
    }

    fn lifecycle_lock(&self, name: &str) -> Result<Arc<AsyncMutex<()>>> {
        let mut locks = self
            .lifecycle_locks
            .lock()
            .map_err(|error| anyhow!("failed to lock plugin lifecycle map: {error}"))?;
        Ok(locks
            .entry(normalize_plugin_name(name))
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone())
    }

    fn unique_managed_path(&self, source_path: &Path, prefix: &str) -> Result<PathBuf> {
        let file_name = source_path
            .file_name()
            .ok_or_else(|| anyhow!("extension path has no file name"))?
            .to_string_lossy();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let counter = UNIQUE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "{prefix}{}-{timestamp}-{counter}-{file_name}",
            std::process::id()
        );
        Ok(source_path.parent().unwrap_or(&self.dir).join(name))
    }

    fn stage_library(&self, plugin_path: &Path) -> Result<PathBuf> {
        let staged_path = self.unique_managed_path(plugin_path, STAGED_LIBRARY_PREFIX)?;
        if let Err(error) = std::fs::copy(plugin_path, &staged_path) {
            let _ = std::fs::remove_file(&staged_path);
            return Err(error.into());
        }
        Ok(staged_path)
    }

    fn cleanup_managed_file(path: &Path) {
        if let Err(error) = std::fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                "failed to remove managed extension file {}: {error}",
                path.display()
            );
        }
    }

    async fn replace_plugin_file(
        &self,
        temporary_path: &Path,
        plugin_path: &Path,
    ) -> Result<PluginFileReplacement> {
        let backup_path = match tokio::fs::metadata(plugin_path).await {
            Ok(_) => {
                let backup_path = self.unique_managed_path(plugin_path, INSTALL_BACKUP_PREFIX)?;
                tokio::fs::rename(plugin_path, &backup_path).await?;
                Some(backup_path)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };

        if let Err(error) = tokio::fs::rename(temporary_path, plugin_path).await {
            if let Some(backup_path) = backup_path.as_ref()
                && let Err(restore_error) = tokio::fs::rename(backup_path, plugin_path).await
            {
                return Err(anyhow!(
                    "failed to replace extension {}: {error}; failed to restore the previous file: {restore_error}",
                    plugin_path.display()
                ));
            }
            return Err(error.into());
        }

        Ok(PluginFileReplacement { backup_path })
    }

    async fn rollback_plugin_file(
        &self,
        plugin_path: &Path,
        replacement: PluginFileReplacement,
    ) -> Result<()> {
        match tokio::fs::remove_file(plugin_path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        if let Some(backup_path) = replacement.backup_path {
            tokio::fs::rename(backup_path, plugin_path).await?;
        }
        Ok(())
    }

    fn read(&self) -> Result<RwLockReadGuard<'_, FnvHashMap<i64, Arc<SourceEntry>>>> {
        self.extensions
            .read()
            .map_err(|e| anyhow!("failed to lock read: {e}"))
    }

    fn write(&self) -> Result<RwLockWriteGuard<'_, FnvHashMap<i64, Arc<SourceEntry>>>> {
        self.extensions
            .write()
            .map_err(|e| anyhow!("failed to lock write: {e}"))
    }

    fn entry(&self, source_id: i64) -> Result<Arc<SourceEntry>> {
        self.read()?
            .get(&source_id)
            .cloned()
            .ok_or_else(|| anyhow!("no such source"))
    }

    async fn acquire_permit(
        &self,
        entry: &Arc<SourceEntry>,
        operation: &'static str,
    ) -> Result<OwnedSemaphorePermit> {
        self.ensure_source_available(entry, operation)?;
        let source_id = entry.source_id;
        let source_name = entry.source_name().to_owned();
        match tokio::time::timeout(
            self.options.admission_timeout,
            entry.limiter.clone().acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => {
                self.ensure_source_available(entry, operation)?;
                Ok(permit)
            }
            Ok(Err(error)) => {
                error!(
                    "EXTENSION ADMISSION ERROR: source_id={source_id} source={source_name:?} operation={operation} limiter unavailable: {error}"
                );
                bail!(
                    "[extension-admission] source {source_id} ({source_name}) cannot accept {operation}: {error}"
                );
            }
            Err(_) => {
                error!(
                    "EXTENSION SATURATION: source_id={source_id} source={source_name:?} operation={operation} exceeded the {}-call limit; request rejected after {:?}",
                    self.options.max_concurrent_calls, self.options.admission_timeout
                );
                bail!(
                    "[extension-saturated] source {source_id} ({source_name}) is busy; {operation} admission timed out after {:?}",
                    self.options.admission_timeout
                );
            }
        }
    }

    fn ensure_source_available(
        &self,
        entry: &Arc<SourceEntry>,
        operation: &'static str,
    ) -> Result<()> {
        let source_id = entry.source_id;
        let source_name = entry.source_name();
        match entry.health.admission() {
            SourceAdmission::Allowed => Ok(()),
            SourceAdmission::CircuitOpen { abandoned_calls } => {
                error!(
                    "EXTENSION CIRCUIT OPEN: source_id={source_id} source={source_name:?} operation={operation} has {abandoned_calls} timed-out native calls still running; rejecting new work"
                );
                bail!(
                    "[extension-circuit-open] source {source_id} ({source_name}) has {abandoned_calls} timed-out native calls still running; replace or restart the source before retrying {operation}"
                );
            }
            SourceAdmission::Quarantined => {
                error!(
                    "EXTENSION QUARANTINED: source_id={source_id} source={source_name:?} operation={operation}; rejecting new work until replacement or reload"
                );
                bail!(
                    "[extension-quarantined] source {source_id} ({source_name}) is quarantined; replace or reload the extension before retrying {operation}"
                );
            }
        }
    }

    async fn call_blocking<T, F>(
        &self,
        source_id: i64,
        call: ExtensionCall,
        request: WorkerRequest,
        decode: impl FnOnce(WorkerValue) -> Result<T> + Send + 'static,
        invoke: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
    {
        let entry = self.entry(source_id)?;
        self.call_blocking_entry(entry, call, request, decode, invoke)
            .await
    }

    async fn call_blocking_entry<T, F, D>(
        &self,
        entry: Arc<SourceEntry>,
        call: ExtensionCall,
        request: WorkerRequest,
        decode: D,
        invoke: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
        self.call_blocking_with(entry, call, request, decode, move |entry| {
            entry.with_extension(invoke)
        })
        .await
    }

    async fn call_blocking_mut_entry<T, F, D>(
        &self,
        entry: Arc<SourceEntry>,
        call: ExtensionCall,
        request: WorkerRequest,
        decode: D,
        invoke: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
        self.call_blocking_with(entry, call, request, decode, move |entry| {
            entry.with_extension_mut(invoke)
        })
        .await
    }

    async fn call_blocking_with<T, F, D>(
        &self,
        entry: Arc<SourceEntry>,
        call: ExtensionCall,
        request: WorkerRequest,
        decode: D,
        invoke: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&SourceEntry) -> Result<T> + Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
        let ExtensionCall {
            operation,
            timeout,
            quarantine_on_panic,
        } = call;
        let permit = self.acquire_permit(&entry, operation).await?;
        if let Some(worker) = entry.worker() {
            return self
                .call_worker(entry, call, worker, permit, request, decode)
                .await;
        }

        drop(request);
        drop(decode);
        let source_id = entry.source_id;
        let source_name = entry.source_name().to_owned();
        let health = entry.health.clone();
        let task_health = health.clone();
        let task_source_name = source_name.clone();
        let tracker = AbandonedCallTracker::new(health.clone());
        let task_tracker = tracker.clone();
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let result = match std::panic::catch_unwind(AssertUnwindSafe(|| invoke(&entry))) {
                Ok(result) => result,
                Err(payload) => {
                    let message = panic_payload_message(&*payload);
                    let quarantined = if quarantine_on_panic {
                        task_health.quarantine();
                        true
                    } else {
                        task_health.record_read_panic()
                    };
                    error!(
                        "EXTENSION PANIC: source_id={source_id} source={task_source_name:?} operation={operation} payload={message:?}"
                    );
                    if quarantined {
                        error!(
                            "EXTENSION QUARANTINED: source_id={source_id} source={task_source_name:?} operation={operation}; replace or reload the extension before retrying"
                        );
                    }
                    Err(anyhow!(
                        "[extension-panicked] source {source_id} ({task_source_name}) {operation} panicked: {message}"
                    ))
                }
            };
            task_tracker.complete();
            result
        });
        match tokio::time::timeout(timeout, task).await {
            Ok(Ok(Ok(value))) => {
                health.record_success();
                Ok(value)
            }
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(join_error)) => {
                if join_error.is_panic() {
                    let quarantined = if quarantine_on_panic {
                        health.quarantine();
                        true
                    } else {
                        health.record_read_panic()
                    };
                    error!(
                        "EXTENSION PANIC: source_id={source_id} source={source_name:?} operation={operation} blocking task panicked outside the extension boundary"
                    );
                    if quarantined {
                        error!(
                            "EXTENSION QUARANTINED: source_id={source_id} source={source_name:?} operation={operation}; replace or reload the extension before retrying"
                        );
                    }
                    bail!(
                        "[extension-panicked] source {source_id} ({source_name}) {operation} panicked"
                    );
                }
                bail!(
                    "[extension-cancelled] source {source_id} ({source_name}) {operation} task was cancelled"
                );
            }
            Err(_) => {
                health.record_timeout();
                let abandoned_calls = tracker.mark_abandoned();
                if let Some(abandoned_calls) = abandoned_calls {
                    error!(
                        "EXTENSION ABANDONED CALL: source_id={source_id} source={source_name:?} operation={operation} remains active after timeout; abandoned_calls={abandoned_calls}"
                    );
                    if abandoned_calls >= SOURCE_MAX_ABANDONED_CALLS {
                        error!(
                            "EXTENSION CIRCUIT OPEN: source_id={source_id} source={source_name:?} operation={operation}; {abandoned_calls} timed-out native calls remain active and future calls are rejected"
                        );
                    }
                }
                error!(
                    "EXTENSION TIMEOUT: source_id={source_id} source={source_name:?} operation={operation} exceeded {timeout:?}; native call may still be running and its permit remains held"
                );
                bail!(
                    "[extension-timeout] source {source_id} ({source_name}) {operation} exceeded {timeout:?}; native call may still be running"
                );
            }
        }
    }

    async fn call_worker<T, D>(
        &self,
        entry: Arc<SourceEntry>,
        call: ExtensionCall,
        worker: Arc<WorkerClient>,
        permit: OwnedSemaphorePermit,
        request: WorkerRequest,
        decode: D,
    ) -> Result<T>
    where
        T: Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
        let join_source_id = entry.source_id;
        let join_source_name = entry.source_name().to_owned();
        let join_operation = call.operation;
        // The supervisor task, rather than the caller future, owns the permit
        // and IPC transaction. If an HTTP request or other caller is
        // cancelled, the supervisor still drains the matching response (or
        // terminates the worker at the deadline) before releasing either.
        Self::await_worker_supervisor(
            join_source_id,
            join_source_name,
            join_operation,
            Self::call_worker_inner(entry, call, worker, permit, request, decode),
        )
        .await
    }

    async fn await_worker_supervisor<T, F>(
        source_id: i64,
        source_name: String,
        operation: &'static str,
        future: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: Future<Output = Result<T>> + Send + 'static,
    {
        let task = tokio::spawn(future);
        match task.await {
            Ok(result) => result,
            Err(error) if error.is_panic() => {
                error!(
                    "EXTENSION WORKER SUPERVISOR PANIC: source_id={source_id} source={source_name:?} operation={operation}"
                );
                bail!(
                    "[extension-worker-supervisor] source {source_id} ({source_name}) {operation} supervisor panicked"
                );
            }
            Err(error) => bail!(
                "[extension-worker-supervisor] source {source_id} ({source_name}) {operation} supervisor was cancelled: {error}"
            ),
        }
    }

    async fn call_worker_inner<T, D>(
        entry: Arc<SourceEntry>,
        call: ExtensionCall,
        worker: Arc<WorkerClient>,
        permit: OwnedSemaphorePermit,
        request: WorkerRequest,
        decode: D,
    ) -> Result<T>
    where
        T: Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
        let ExtensionCall {
            operation,
            timeout,
            quarantine_on_panic,
        } = call;
        let source_id = entry.source_id;
        let source_name = entry.source_name().to_owned();
        let health = entry.health.clone();
        let result = worker.request(request, timeout).await;
        drop(permit);

        match result {
            Ok(value) => {
                let value = decode(value).map_err(|error| {
                    error!(
                        "EXTENSION WORKER PROTOCOL ERROR: source_id={source_id} source={source_name:?} operation={operation} response={error}"
                    );
                    anyhow!(
                        "[extension-worker-protocol] source {source_id} ({source_name}) {operation} returned an invalid response: {error}"
                    )
                })?;
                health.record_success();
                Ok(value)
            }
            Err(WorkerCallError::Remote {
                kind: WorkerErrorKind::Operation,
                message,
            }) => Err(anyhow!(
                "[extension-error] source {source_id} ({source_name}) {operation} failed: {message}"
            )),
            // The request never reached the worker, so it says nothing about
            // the source's health; the head-of-line call reports for both.
            Err(WorkerCallError::QueueTimeout) => {
                warn!(
                    "EXTENSION WORKER BUSY: source_id={source_id} source={source_name:?} operation={operation} spent {timeout:?} waiting behind earlier calls; the worker was not disturbed"
                );
                Err(anyhow!(
                    "[extension-worker-busy] source {source_id} ({source_name}) is busy; {operation} timed out after {timeout:?} waiting for earlier calls"
                ))
            }
            // The source was unloaded or replaced mid-call; the retired
            // entry's health no longer matters.
            Err(WorkerCallError::Stopped) => {
                warn!(
                    "EXTENSION WORKER STOPPED: source_id={source_id} source={source_name:?} operation={operation} was interrupted because the source was unloaded or replaced"
                );
                Err(anyhow!(
                    "[extension-stopped] source {source_id} ({source_name}) {operation} was interrupted because the source was unloaded or replaced"
                ))
            }
            Err(error) => {
                let (kind, message, quarantine) = match error {
                    WorkerCallError::Timeout => (
                        "TIMEOUT",
                        format!("exceeded {timeout:?}; the worker was terminated"),
                        false,
                    ),
                    WorkerCallError::Crashed(message) => ("CRASH", message, quarantine_on_panic),
                    WorkerCallError::Remote { kind, message } => {
                        ("PANIC", format!("{kind:?}: {message}"), quarantine_on_panic)
                    }
                    WorkerCallError::QueueTimeout | WorkerCallError::Stopped => {
                        unreachable!("handled above")
                    }
                };
                let quarantined = if quarantine {
                    health.quarantine();
                    true
                } else {
                    health.record_failure()
                };
                error!(
                    "EXTENSION WORKER {kind}: source_id={source_id} source={source_name:?} operation={operation} {message}"
                );
                if quarantined {
                    error!(
                        "EXTENSION QUARANTINED: source_id={source_id} source={source_name:?} operation={operation}; replace or reload the extension before retrying"
                    );
                }
                let error_kind = if kind == "TIMEOUT" {
                    "extension-worker-timeout"
                } else if kind == "PANIC" {
                    "extension-panicked"
                } else {
                    "extension-worker-crashed"
                };
                bail!(
                    "[{error_kind}] source {source_id} ({source_name}) {operation} failed: {message}"
                );
            }
        }
    }

    pub async fn load_all(&self) -> Result<()> {
        let mut read_dir = tokio::fs::read_dir(&self.dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(PLUGIN_EXTENSION)
                && !is_managed_library_name(&name)
                && let Err(e) = self.load(&name).await
            {
                error!("failed to load {name}: {e}");
            }
        }
        Ok(())
    }

    pub async fn exists(&self, source_id: i64) -> Result<bool> {
        Ok(self.read()?.get(&source_id).is_some())
    }

    pub async fn list(&self) -> Result<Vec<SourceInfo>> {
        let entries = self.read()?.values().cloned().collect::<Vec<_>>();
        Ok(entries
            .into_iter()
            .map(|entry| entry.source_info.clone())
            .collect())
    }

    pub async fn install(&self, repo_url: &str, name: &str) -> Result<()> {
        let plugin_name = normalize_plugin_name(name);
        let source_file_url = format!(
            "{}/{}/{}.{}",
            repo_url,
            env!("TARGET"),
            plugin_name,
            PLUGIN_EXTENSION
        );

        info!("downloading {source_file_url}");

        let contents = reqwest::get(&source_file_url).await?.bytes().await?;
        let plugin_path = self.dir.join(&plugin_name).with_extension(PLUGIN_EXTENSION);
        let temporary_path = self.unique_managed_path(&plugin_path, INSTALL_TEMP_PREFIX)?;

        if let Err(error) = tokio::fs::write(&temporary_path, contents).await {
            Self::cleanup_managed_file(&temporary_path);
            return Err(error.into());
        }
        let lifecycle_lock = self.lifecycle_lock(&plugin_name)?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;

        let validation_entry = match self
            .load_worker_from_path(&temporary_path, plugin_path.clone())
            .await
        {
            Ok(entry) => entry,
            Err(error) => {
                Self::cleanup_managed_file(&temporary_path);
                return Err(error);
            }
        };
        if let Some(worker) = validation_entry.worker() {
            worker.shutdown().await;
        }
        drop(validation_entry);

        // Resolve the current registration before mutating the plugin file.
        // Keep its staged worker serving until the replacement is ready, then
        // pause it only for the map swap below.
        let previous = self.entry_for_plugin(&plugin_name)?;
        let previous_worker = previous.as_ref().and_then(|entry| entry.worker());

        let replacement = match self
            .replace_plugin_file(&temporary_path, &plugin_path)
            .await
        {
            Ok(replacement) => replacement,
            Err(error) => {
                Self::cleanup_managed_file(&temporary_path);
                return Err(error);
            }
        };

        let entry = match self
            .load_worker_from_path(&plugin_path, plugin_path.clone())
            .await
        {
            Ok(entry) => entry,
            Err(error) => {
                if let Err(rollback_error) =
                    self.rollback_plugin_file(&plugin_path, replacement).await
                {
                    return Err(anyhow!(
                        "failed to load the installed extension: {error}; failed to restore the previous file: {rollback_error}"
                    ));
                }
                return Err(error);
            }
        };

        if let Err(error) = self.apply_saved_preferences(&entry).await {
            if let Some(worker) = entry.worker() {
                worker.shutdown().await;
            }
            if let Err(rollback_error) = self.rollback_plugin_file(&plugin_path, replacement).await
            {
                return Err(anyhow!(
                    "failed to apply saved preferences to the installed extension: {error}; failed to restore the previous file: {rollback_error}"
                ));
            }
            return Err(error);
        }

        // Only pause the previous worker for this plugin right before the
        // atomic map swap below, so it keeps serving calls through the new
        // worker's spawn, validation, and preference replay above. A
        // failure past this point resumes the same staged worker,
        // preserving the previous registration.
        if let Some(worker) = previous_worker.as_ref() {
            worker.pause().await;
        }

        let retired = match self.replace_plugin_entry(entry.clone(), previous.as_ref()) {
            Ok(retired) => retired,
            Err(error) => {
                if let Some(worker) = entry.worker() {
                    worker.shutdown().await;
                }
                if let Some(worker) = previous_worker.as_ref() {
                    worker.resume();
                }
                if let Err(rollback_error) =
                    self.rollback_plugin_file(&plugin_path, replacement).await
                {
                    return Err(anyhow!(
                        "failed to register extension: {error}; failed to restore the previous file: {rollback_error}"
                    ));
                }
                return Err(error);
            }
        };
        for entry in retired {
            if let Some(worker) = entry.worker() {
                worker.shutdown().await;
            }
        }

        if let Some(backup_path) = replacement.backup_path {
            Self::cleanup_managed_file(&backup_path);
        }

        info!("installed extension {name}");

        Ok(())
    }

    async fn load_worker_from_path(
        &self,
        source_path: &Path,
        plugin_path: PathBuf,
    ) -> Result<Arc<SourceEntry>> {
        let staged_path = self.stage_library(source_path)?;
        info!(
            "load {:?} from {:?}",
            staged_path.display(),
            plugin_path.display()
        );

        #[cfg(target_os = "macos")]
        if let Err(error) = std::process::Command::new("install_name_tool")
            .current_dir(staged_path.parent().unwrap())
            .arg("-id")
            .arg("''")
            .arg(staged_path.file_name().unwrap())
            .output()
        {
            error!("failed to run install_name_tool: {error}");
        }

        let worker = WorkerClient::new(
            staged_path.clone(),
            self.worker_path.clone(),
            self.options.metadata_timeout,
            Some(staged_path.clone()),
        );
        let source_info = match worker.start().await {
            Ok(source_info) => source_info,
            Err(error) => {
                worker.shutdown().await;
                return Err(anyhow!(
                    "failed to start extension worker for {}: {error}",
                    plugin_path.display()
                ));
            }
        };

        Ok(Arc::new(SourceEntry::from_worker(
            source_info,
            worker,
            self.options.max_concurrent_calls,
            plugin_path,
            tanoshi_lib::RUSTC_VERSION.to_string(),
            tanoshi_lib::LIB_VERSION.to_string(),
        )))
    }

    /// Applies the source's persisted preferences to a freshly loaded entry
    /// before it is published in the source map.
    async fn apply_saved_preferences(&self, entry: &Arc<SourceEntry>) -> Result<()> {
        let source_name = entry.source_info.name.to_lowercase();
        let Some(preferences) =
            tokio::fs::read_to_string(self.dir.join(&source_name).with_extension("json"))
                .await
                .ok()
                .and_then(|s| serde_json::from_str::<Vec<Input>>(&s).ok())
        else {
            return Ok(());
        };

        info!("set preferences for {source_name}");
        let saved_preferences = preferences.clone();
        self.call_blocking_mut_entry(
            entry.clone(),
            ExtensionCall {
                operation: "load_preferences",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: true,
            },
            WorkerRequest::SetPreferences {
                preferences: preferences.clone(),
            },
            decode_unit,
            move |extension| extension.set_preferences(preferences),
        )
        .await?;
        if let Some(worker) = entry.worker() {
            worker.set_startup_preferences(saved_preferences);
        }
        Ok(())
    }

    pub async fn load(&self, name: &str) -> Result<()> {
        let plugin_name = normalize_plugin_name(name);
        let lifecycle_lock = self.lifecycle_lock(&plugin_name)?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;
        let plugin_path = self.dir.join(&plugin_name).with_extension(PLUGIN_EXTENSION);
        let entry = self
            .load_worker_from_path(&plugin_path, plugin_path.clone())
            .await?;

        if let Err(error) = self.apply_saved_preferences(&entry).await {
            if let Some(worker) = entry.worker() {
                worker.shutdown().await;
            }
            return Err(error);
        }
        let replaced = self.insert_entry(entry)?;
        if let Some(worker) = replaced.as_ref().and_then(|entry| entry.worker()) {
            worker.shutdown().await;
        }
        Ok(())
    }

    pub async fn insert(&self, source: Source) -> Result<()> {
        let entry = Arc::new(source.into_entry(self.options.max_concurrent_calls)?);
        let lifecycle_lock = self.lifecycle_lock(&entry_plugin_name(&entry))?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;
        let replaced = self.insert_entry(entry)?;
        if let Some(worker) = replaced.as_ref().and_then(|entry| entry.worker()) {
            worker.shutdown().await;
        }
        Ok(())
    }

    fn insert_entry(&self, entry: Arc<SourceEntry>) -> Result<Option<Arc<SourceEntry>>> {
        let source_id = entry.source_id;
        Ok({
            let mut sources = self.write()?;
            sources.insert(source_id, entry)
        })
    }

    fn entry_for_plugin(&self, plugin_name: &str) -> Result<Option<Arc<SourceEntry>>> {
        Ok(self
            .read()?
            .values()
            .find(|entry| entry_plugin_name(entry) == plugin_name)
            .cloned())
    }

    fn replace_plugin_entry(
        &self,
        entry: Arc<SourceEntry>,
        previous: Option<&Arc<SourceEntry>>,
    ) -> Result<Vec<Arc<SourceEntry>>> {
        let mut retired = Vec::new();
        let mut sources = self.write()?;

        if let Some(previous) = previous
            && sources
                .get(&previous.source_id)
                .is_some_and(|current| Arc::ptr_eq(current, previous))
            && let Some(previous) = sources.remove(&previous.source_id)
        {
            retired.push(previous);
        }

        if let Some(replaced) = sources.insert(entry.source_id, entry)
            && !retired.iter().any(|entry| Arc::ptr_eq(entry, &replaced))
        {
            retired.push(replaced);
        }

        Ok(retired)
    }

    pub async fn unload(&self, source_id: i64) -> Result<()> {
        loop {
            let entry = match self.read()?.get(&source_id).cloned() {
                Some(entry) => entry,
                None => return Ok(()),
            };
            let plugin_name = entry_plugin_name(&entry);
            let lifecycle_lock = self.lifecycle_lock(&plugin_name)?;
            let _lifecycle_guard = lifecycle_lock.lock_owned().await;
            let Some(entry) = self.read()?.get(&source_id).cloned() else {
                return Ok(());
            };
            if entry_plugin_name(&entry) != plugin_name {
                continue;
            }

            let worker = entry.worker();
            if let Some(worker) = worker.as_ref() {
                // Keep the registration present but stopped until physical
                // deletion succeeds. If deletion fails, resume this worker so
                // the Phase 1 unload-failure behavior remains transactional.
                worker.pause().await;
            }

            if let Some(plugin_path) = entry.plugin_path()
                && let Err(error) = std::fs::remove_file(plugin_path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                if let Some(worker) = worker.as_ref() {
                    worker.resume();
                }
                return Err(anyhow!(
                    "failed to remove extension {}: {error}",
                    plugin_path.display()
                ));
            }

            let removed = {
                let mut sources = self.write()?;
                if sources
                    .get(&source_id)
                    .is_some_and(|current| Arc::ptr_eq(current, &entry))
                {
                    sources.remove(&source_id)
                } else {
                    None
                }
            };
            if removed.is_none() {
                if let Some(worker) = worker.as_ref() {
                    worker.shutdown().await;
                }
                continue;
            }
            if let Some(worker) = worker {
                worker.shutdown().await;
            }
            info!(
                "uninstalled extension {source_id} ({})",
                entry.source_name()
            );
            return Ok(());
        }
    }

    pub async fn remove(&self, source_id: i64) -> Result<()> {
        self.unload(source_id).await
    }

    pub fn get_version(&self, source_id: i64) -> Result<(String, String)> {
        let sources = self.read()?;
        let source = sources
            .get(&source_id)
            .ok_or_else(|| anyhow!("no such source"))?;
        let rustc_version = source.rustc_version.clone();
        let lib_version = source.lib_version.clone();
        Ok((rustc_version, lib_version))
    }

    pub fn get_source_info(&self, source_id: i64) -> Result<SourceInfo> {
        let entry = self.read()?.get(&source_id).cloned();
        if let Some(entry) = entry {
            Ok(entry.source_info.clone())
        } else {
            println!("Returning dummy source info");
            Ok(dummy_source_info(source_id))
        }
    }

    pub async fn filter_list(&self, source_id: i64) -> Result<Vec<Input>> {
        let entry = self.entry(source_id)?;
        self.call_blocking_entry(
            entry,
            ExtensionCall {
                operation: "filter_list",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::FilterList,
            decode_inputs,
            |extension| Ok(extension.filter_list()),
        )
        .await
    }

    pub async fn get_preferences(&self, source_id: i64) -> Result<Vec<Input>> {
        let entry = self.entry(source_id)?;
        self.call_blocking_entry(
            entry,
            ExtensionCall {
                operation: "get_preferences",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::GetPreferences,
            decode_inputs,
            |extension| extension.get_preferences(),
        )
        .await
    }

    pub async fn set_preferences(&self, source_id: i64, preferences: Vec<Input>) -> Result<()> {
        let initial_entry = self.entry(source_id)?;
        let plugin_name = entry_plugin_name(&initial_entry);
        drop(initial_entry);

        // The task owns the lifecycle guard so caller cancellation cannot
        // let an install pass the save while its worker request is still in
        // flight. Dropping the JoinHandle detaches rather than cancels it.
        let manager = self.clone();
        let task = tokio::spawn(async move {
            manager
                .set_preferences_for_plugin(plugin_name, preferences)
                .await
        });
        task.await
            .map_err(|error| anyhow!("preference save task failed: {error}"))?
    }

    async fn set_preferences_for_plugin(
        &self,
        plugin_name: String,
        preferences: Vec<Input>,
    ) -> Result<()> {
        let lifecycle_lock = self.lifecycle_lock(&plugin_name)?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;
        let entry = self
            .entry_for_plugin(&plugin_name)?
            .ok_or_else(|| anyhow!("no such source"))?;
        let source_name = entry.source_info.name.to_lowercase();
        let extension_preferences = preferences.clone();
        self.call_blocking_mut_entry(
            entry.clone(),
            ExtensionCall {
                operation: "set_preferences",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: true,
            },
            WorkerRequest::SetPreferences {
                preferences: extension_preferences.clone(),
            },
            decode_unit,
            move |extension| extension.set_preferences(extension_preferences),
        )
        .await?;
        if let Some(worker) = entry.worker() {
            worker.set_startup_preferences(preferences.clone());
        }

        tokio::fs::write(
            self.dir.join(source_name).with_extension("json"),
            serde_json::to_string_pretty(&preferences)?,
        )
        .await?;

        Ok(())
    }

    pub async fn get_popular_manga(
        &self,
        source_id: i64,
        page: i64,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        self.call_blocking(
            source_id,
            ExtensionCall {
                operation: "get_popular_manga",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::GetPopularManga { page },
            decode_manga_list,
            move |extension| extension.get_popular_manga(page),
        )
        .await
    }

    pub async fn get_latest_manga(
        &self,
        source_id: i64,
        page: i64,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        self.call_blocking(
            source_id,
            ExtensionCall {
                operation: "get_latest_manga",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::GetLatestManga { page },
            decode_manga_list,
            move |extension| extension.get_latest_manga(page),
        )
        .await
    }

    pub async fn search_manga(
        &self,
        source_id: i64,
        page: i64,
        query: Option<String>,
        filters: Option<Vec<Input>>,
    ) -> Result<Vec<tanoshi_lib::prelude::MangaInfo>> {
        self.call_blocking(
            source_id,
            ExtensionCall {
                operation: "search_manga",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::SearchManga {
                page,
                query: query.clone(),
                filters: filters.clone(),
            },
            decode_manga_list,
            move |extension| extension.search_manga(page, query, filters),
        )
        .await
    }

    pub async fn get_manga_detail(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<tanoshi_lib::prelude::MangaInfo> {
        self.call_blocking(
            source_id,
            ExtensionCall {
                operation: "get_manga_detail",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::GetMangaDetail { path: path.clone() },
            decode_manga,
            move |extension| extension.get_manga_detail(path),
        )
        .await
    }

    pub async fn get_chapters(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<tanoshi_lib::prelude::ChapterInfo>> {
        self.call_blocking(
            source_id,
            ExtensionCall {
                operation: "get_chapters",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::GetChapters { path: path.clone() },
            decode_chapters,
            move |extension| extension.get_chapters(path),
        )
        .await
    }

    pub async fn get_pages(&self, source_id: i64, path: String) -> Result<Vec<String>> {
        self.call_blocking(
            source_id,
            ExtensionCall {
                operation: "get_pages",
                timeout: self.options.metadata_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::GetPages { path: path.clone() },
            decode_pages,
            move |extension| extension.get_pages(path),
        )
        .await
    }

    pub async fn get_image_bytes(&self, source_id: i64, url: String) -> Result<Bytes> {
        self.call_blocking(
            source_id,
            ExtensionCall {
                operation: "get_image_bytes",
                timeout: self.options.image_timeout,
                quarantine_on_panic: false,
            },
            WorkerRequest::GetImageBytes { url: url.clone() },
            decode_image,
            move |extension| extension.get_image_bytes(url),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context, Poll, Waker},
        time::Duration,
    };

    use anyhow::{Result, bail};
    use bytes::Bytes;
    use tanoshi_lib::prelude::{ChapterInfo, Extension, Input, Lang, MangaInfo, SourceInfo};
    use tokio::sync::{Semaphore, oneshot};

    use crate::prelude::Source;

    use super::{ExtensionManager, UNIQUE_PATH_COUNTER};

    struct PreferenceExtension {
        source_id: i64,
        calls: Arc<AtomicUsize>,
    }

    impl Extension for PreferenceExtension {
        fn get_source_info(&self) -> SourceInfo {
            SourceInfo {
                id: self.source_id,
                name: "Race Source".to_string(),
                url: String::new(),
                version: "test",
                icon: "",
                languages: Lang::All,
                nsfw: false,
            }
        }

        fn set_preferences(&mut self, _preferences: Vec<Input>) -> Result<()> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn get_popular_manga(&self, _page: i64) -> Result<Vec<MangaInfo>> {
            bail!("unused test operation")
        }

        fn get_latest_manga(&self, _page: i64) -> Result<Vec<MangaInfo>> {
            bail!("unused test operation")
        }

        fn search_manga(
            &self,
            _page: i64,
            _query: Option<String>,
            _filters: Option<Vec<Input>>,
        ) -> Result<Vec<MangaInfo>> {
            bail!("unused test operation")
        }

        fn get_manga_detail(&self, _path: String) -> Result<MangaInfo> {
            bail!("unused test operation")
        }

        fn get_chapters(&self, _path: String) -> Result<Vec<ChapterInfo>> {
            bail!("unused test operation")
        }

        fn get_pages(&self, _path: String) -> Result<Vec<String>> {
            bail!("unused test operation")
        }

        fn get_image_bytes(&self, _url: String) -> Result<Bytes> {
            bail!("unused test operation")
        }
    }

    fn preference_entry(
        source_id: i64,
        calls: Arc<AtomicUsize>,
        plugin_path: &std::path::Path,
    ) -> Arc<crate::prelude::SourceEntry> {
        Arc::new(
            Source::from(Box::new(PreferenceExtension { source_id, calls }))
                .with_plugin_path(plugin_path.to_path_buf())
                .into_entry(1)
                .unwrap(),
        )
    }

    #[tokio::test]
    async fn cancelled_caller_does_not_drop_supervised_resources() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();

        let caller = tokio::spawn(ExtensionManager::await_worker_supervisor(
            1,
            "test source".to_string(),
            "test_operation",
            async move {
                let _ = started_tx.send(());
                let _ = release_rx.await;
                drop(permit);
                Ok(())
            },
        ));
        started_rx.await.unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());

        assert!(
            tokio::time::timeout(Duration::from_millis(20), semaphore.clone().acquire_owned(),)
                .await
                .is_err(),
            "caller cancellation released the supervised permit early"
        );

        release_tx.send(()).unwrap();
        let recovered_permit =
            tokio::time::timeout(Duration::from_secs(1), semaphore.acquire_owned())
                .await
                .expect("supervised permit was not released after completion")
                .unwrap();
        drop(recovered_permit);
    }

    #[tokio::test]
    async fn preference_save_waits_for_install_and_targets_replacement_entry() {
        let test_id = UNIQUE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-vm-preference-install-race-{}-{test_id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let manager = ExtensionManager::new(&dir);
        let plugin_path = dir.join("race.so");
        let old_calls = Arc::new(AtomicUsize::new(0));
        let old_entry = preference_entry(1, old_calls.clone(), &plugin_path);
        manager.insert_entry(old_entry.clone()).unwrap();

        let lifecycle_lock = manager.lifecycle_lock("race").unwrap();
        let lifecycle_guard = lifecycle_lock.lock_owned().await;
        let preferences = vec![Input::Text {
            name: "token".to_string(),
            state: Some("new value".to_string()),
        }];
        let mut preference_save = Box::pin(manager.set_preferences(1, preferences));
        let mut context = Context::from_waker(Waker::noop());
        assert!(
            matches!(preference_save.as_mut().poll(&mut context), Poll::Pending),
            "preference save did not wait for the plugin lifecycle lock"
        );

        let new_calls = Arc::new(AtomicUsize::new(0));
        let new_entry = preference_entry(2, new_calls.clone(), &plugin_path);
        manager
            .replace_plugin_entry(new_entry, Some(&old_entry))
            .unwrap();
        drop(lifecycle_guard);

        preference_save.await.unwrap();
        assert_eq!(old_calls.load(Ordering::Relaxed), 0);
        assert_eq!(new_calls.load(Ordering::Relaxed), 1);
        assert!(dir.join("race source.json").is_file());

        std::fs::remove_dir_all(dir).unwrap();
    }
}
