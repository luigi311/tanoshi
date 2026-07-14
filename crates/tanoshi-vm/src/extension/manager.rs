use std::{
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
    WorkerCallError, WorkerClient, WorkerRequest, WorkerValue, resolve_worker_path,
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
        operation: &'static str,
        timeout: Duration,
        request: WorkerRequest,
        decode: impl FnOnce(WorkerValue) -> Result<T> + Send + 'static,
        call: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
    {
        let entry = self.entry(source_id)?;
        self.call_blocking_entry(entry, operation, timeout, request, decode, call)
            .await
    }

    async fn call_blocking_entry<T, F, D>(
        &self,
        entry: Arc<SourceEntry>,
        operation: &'static str,
        timeout: Duration,
        request: WorkerRequest,
        decode: D,
        call: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
        self.call_blocking_with(
            entry,
            operation,
            timeout,
            false,
            request,
            decode,
            move |entry| entry.with_extension(call),
        )
        .await
    }

    async fn call_blocking_mut_entry<T, F, D>(
        &self,
        entry: Arc<SourceEntry>,
        operation: &'static str,
        timeout: Duration,
        request: WorkerRequest,
        decode: D,
        call: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut dyn tanoshi_lib::prelude::Extension) -> Result<T> + Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
        self.call_blocking_with(
            entry,
            operation,
            timeout,
            true,
            request,
            decode,
            move |entry| entry.with_extension_mut(call),
        )
        .await
    }

    async fn call_blocking_with<T, F, D>(
        &self,
        entry: Arc<SourceEntry>,
        operation: &'static str,
        timeout: Duration,
        quarantine_on_panic: bool,
        request: WorkerRequest,
        decode: D,
        invoke: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&SourceEntry) -> Result<T> + Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
        let permit = self.acquire_permit(&entry, operation).await?;
        if let Some(worker) = entry.worker() {
            return self
                .call_worker(
                    entry,
                    operation,
                    timeout,
                    quarantine_on_panic,
                    worker,
                    permit,
                    request,
                    decode,
                )
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
        operation: &'static str,
        timeout: Duration,
        quarantine_on_panic: bool,
        worker: Arc<WorkerClient>,
        permit: OwnedSemaphorePermit,
        request: WorkerRequest,
        decode: D,
    ) -> Result<T>
    where
        T: Send + 'static,
        D: FnOnce(WorkerValue) -> Result<T> + Send + 'static,
    {
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
            Err(error) => {
                let (kind, message) = match error {
                    WorkerCallError::Timeout => (
                        "TIMEOUT",
                        format!("exceeded {timeout:?}; the worker was terminated"),
                    ),
                    WorkerCallError::Crashed(message) => ("CRASH", message),
                    WorkerCallError::Remote { kind, message }
                        if kind == super::worker::WorkerErrorKind::Operation =>
                    {
                        return Err(anyhow!(
                            "[extension-error] source {source_id} ({source_name}) {operation} failed: {message}"
                        ));
                    }
                    WorkerCallError::Remote { kind, message } => {
                        ("PANIC", format!("{kind:?}: {message}"))
                    }
                };
                let quarantined = if quarantine_on_panic {
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

        if let Err(error) = self.insert_entry(entry) {
            if let Err(rollback_error) = self.rollback_plugin_file(&plugin_path, replacement).await
            {
                return Err(anyhow!(
                    "failed to register extension: {error}; failed to restore the previous file: {rollback_error}"
                ));
            }
            return Err(error);
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

    pub async fn load(&self, name: &str) -> Result<()> {
        let plugin_name = normalize_plugin_name(name);
        let lifecycle_lock = self.lifecycle_lock(&plugin_name)?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;
        let plugin_path = self.dir.join(&plugin_name).with_extension(PLUGIN_EXTENSION);
        let entry = self
            .load_worker_from_path(&plugin_path, plugin_path.clone())
            .await?;
        let source_name = entry.source_info.name.to_lowercase();

        if let Some(preferences) =
            tokio::fs::read_to_string(self.dir.join(&source_name).with_extension("json"))
                .await
                .ok()
                .and_then(|s| serde_json::from_str::<Vec<Input>>(&s).ok())
        {
            info!("set preferences for {}", source_name);
            self.call_blocking_mut_entry(
                entry.clone(),
                "load_preferences",
                self.options.metadata_timeout,
                WorkerRequest::SetPreferences {
                    preferences: preferences.clone(),
                },
                decode_unit,
                move |extension| extension.set_preferences(preferences),
            )
            .await?;
        }
        self.insert_entry(entry)
    }

    pub async fn insert(&self, source: Source) -> Result<()> {
        let entry = Arc::new(source.into_entry(self.options.max_concurrent_calls)?);
        let lifecycle_lock = self.lifecycle_lock(&entry_plugin_name(&entry))?;
        let _lifecycle_guard = lifecycle_lock.lock_owned().await;
        self.insert_entry(entry)
    }

    fn insert_entry(&self, entry: Arc<SourceEntry>) -> Result<()> {
        let source_id = entry.source_id;
        let replaced = {
            let mut sources = self.write()?;
            sources.insert(source_id, entry)
        };
        drop(replaced);
        Ok(())
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
            let (entry, removed) = {
                let mut sources = self.write()?;
                let Some(entry) = sources.get(&source_id).cloned() else {
                    return Ok(());
                };
                if entry_plugin_name(&entry) != plugin_name {
                    continue;
                }

                let plugin_path = entry.plugin_path().map(Path::to_path_buf);
                if let Some(plugin_path) = plugin_path.as_deref()
                    && let Err(error) = std::fs::remove_file(plugin_path)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(anyhow!(
                        "failed to remove extension {}: {error}",
                        plugin_path.display()
                    ));
                }

                let removed = sources.remove(&source_id);
                (entry, removed)
            };
            if let Some(worker) = removed.as_ref().and_then(|entry| entry.worker()) {
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
            "filter_list",
            self.options.metadata_timeout,
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
            "get_preferences",
            self.options.metadata_timeout,
            WorkerRequest::GetPreferences,
            decode_inputs,
            |extension| extension.get_preferences(),
        )
        .await
    }

    pub async fn set_preferences(&self, source_id: i64, preferences: Vec<Input>) -> Result<()> {
        let entry = self.entry(source_id)?;
        let source_name = entry.source_info.name.to_lowercase();
        let extension_preferences = preferences.clone();
        self.call_blocking_mut_entry(
            entry,
            "set_preferences",
            self.options.metadata_timeout,
            WorkerRequest::SetPreferences {
                preferences: extension_preferences.clone(),
            },
            decode_unit,
            move |extension| extension.set_preferences(extension_preferences),
        )
        .await?;

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
            "get_popular_manga",
            self.options.metadata_timeout,
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
            "get_latest_manga",
            self.options.metadata_timeout,
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
            "search_manga",
            self.options.metadata_timeout,
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
            "get_manga_detail",
            self.options.metadata_timeout,
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
            "get_chapters",
            self.options.metadata_timeout,
            WorkerRequest::GetChapters { path: path.clone() },
            decode_chapters,
            move |extension| extension.get_chapters(path),
        )
        .await
    }

    pub async fn get_pages(&self, source_id: i64, path: String) -> Result<Vec<String>> {
        self.call_blocking(
            source_id,
            "get_pages",
            self.options.metadata_timeout,
            WorkerRequest::GetPages { path: path.clone() },
            decode_pages,
            move |extension| extension.get_pages(path),
        )
        .await
    }

    pub async fn get_image_bytes(&self, source_id: i64, url: String) -> Result<Bytes> {
        self.call_blocking(
            source_id,
            "get_image_bytes",
            self.options.image_timeout,
            WorkerRequest::GetImageBytes { url: url.clone() },
            decode_image,
            move |extension| extension.get_image_bytes(url),
        )
        .await
    }
}
