use std::{
    any::Any,
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU8, AtomicU32, AtomicUsize, Ordering},
    },
};

use anyhow::{Result, anyhow};
use libloading::Library;
use once_cell::sync::OnceCell;
use tanoshi_lib::prelude::{Extension, SourceInfo};
use tokio::sync::Semaphore;

use std::panic::{AssertUnwindSafe, catch_unwind};

use super::worker::{WorkerClient, WorkerSourceInfo};

pub(crate) const SOURCE_HEALTH_FAILURE_THRESHOLD: u32 = 3;
pub(crate) const SOURCE_MAX_ABANDONED_CALLS: usize = 3;

const HEALTHY: u8 = 0;
const DEGRADED: u8 = 1;
const QUARANTINED: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceAdmission {
    Allowed,
    CircuitOpen { abandoned_calls: usize },
    Quarantined,
}

pub(crate) struct SourceHealth {
    state: AtomicU8,
    read_panics: AtomicU32,
    abandoned_calls: AtomicUsize,
}

impl SourceHealth {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(HEALTHY),
            read_panics: AtomicU32::new(0),
            abandoned_calls: AtomicUsize::new(0),
        })
    }

    pub(crate) fn admission(&self) -> SourceAdmission {
        if self.state.load(Ordering::Acquire) == QUARANTINED {
            return SourceAdmission::Quarantined;
        }

        let abandoned_calls = self.abandoned_calls.load(Ordering::Acquire);
        if abandoned_calls >= SOURCE_MAX_ABANDONED_CALLS {
            SourceAdmission::CircuitOpen { abandoned_calls }
        } else {
            SourceAdmission::Allowed
        }
    }

    pub(crate) fn quarantine(&self) {
        self.state.store(QUARANTINED, Ordering::Release);
    }

    pub(crate) fn record_read_panic(&self) -> bool {
        self.record_failure()
    }

    pub(crate) fn record_failure(&self) -> bool {
        self.mark_degraded();
        let failures = self
            .read_panics
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        if failures >= SOURCE_HEALTH_FAILURE_THRESHOLD {
            self.quarantine();
            true
        } else {
            false
        }
    }

    pub(crate) fn record_timeout(&self) {
        self.mark_degraded();
    }

    pub(crate) fn start_abandoned_call(&self) -> usize {
        self.mark_degraded();
        self.abandoned_calls
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1)
    }

    pub(crate) fn complete_abandoned_call(&self) {
        let previous = self
            .abandoned_calls
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                count.checked_sub(1)
            })
            .unwrap_or(0);
        if previous == 1
            && self.read_panics.load(Ordering::Acquire) == 0
            && self
                .state
                .compare_exchange(DEGRADED, HEALTHY, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            // A concurrent panic or quarantine wins over automatic recovery.
        }
    }

    pub(crate) fn record_success(&self) {
        self.read_panics.store(0, Ordering::Release);
        if self.abandoned_calls.load(Ordering::Acquire) == 0 {
            let _ =
                self.state
                    .compare_exchange(DEGRADED, HEALTHY, Ordering::AcqRel, Ordering::Acquire);
        }
    }

    fn mark_degraded(&self) {
        let mut state = self.state.load(Ordering::Acquire);
        while state != QUARANTINED {
            match self
                .state
                .compare_exchange(state, DEGRADED, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) | Err(QUARANTINED) => break,
                Err(next) => state = next,
            }
        }
    }
}

pub(crate) fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload");
    const MAX_PANIC_MESSAGE_LENGTH: usize = 256;
    let end = message
        .char_indices()
        .take_while(|(index, _)| *index <= MAX_PANIC_MESSAGE_LENGTH)
        .map(|(index, _)| index)
        .last()
        .unwrap_or(0);
    if message.len() > MAX_PANIC_MESSAGE_LENGTH {
        format!("{}...", &message[..end])
    } else {
        message.to_owned()
    }
}

pub(crate) struct LoadedLibrary {
    library: Option<Library>,
    path: Option<PathBuf>,
}

impl LoadedLibrary {
    fn new(library: Library) -> Self {
        Self {
            library: Some(library),
            path: None,
        }
    }
}

impl Drop for LoadedLibrary {
    fn drop(&mut self) {
        let library = self.library.take();
        drop(library);

        if let Some(path) = self.path.take()
            && let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!(
                "failed to remove staged extension library {}: {error}; retrying at startup",
                path.display()
            );
        }
    }
}

/// A loaded source that can outlive its entry in the manager's source map.
pub struct SourceEntry {
    pub(crate) source_id: i64,
    pub(crate) source_info: SourceInfo,
    // Keep the in-process extension before the library. Struct fields are
    // dropped in declaration order, so the extension is destroyed before its
    // library.
    pub(crate) extension: Option<RwLock<Box<dyn Extension>>>,
    pub(crate) worker: Option<Arc<WorkerClient>>,
    pub(crate) limiter: Arc<Semaphore>,
    pub(crate) health: Arc<SourceHealth>,
    #[allow(dead_code)]
    pub(crate) library: Option<LoadedLibrary>,
    pub(crate) plugin_path: Option<PathBuf>,
    pub(crate) rustc_version: String,
    pub(crate) lib_version: String,
}

impl SourceEntry {
    pub(crate) fn with_extension<T>(
        &self,
        call: impl FnOnce(&dyn Extension) -> Result<T>,
    ) -> Result<T> {
        let extension = self
            .extension
            .as_ref()
            .ok_or_else(|| anyhow!("source {} is hosted by an extension worker", self.source_id))?
            .read()
            .map_err(|error| anyhow!("source {} read lock is poisoned: {error}", self.source_id))?;
        call(extension.as_ref())
    }

    pub(crate) fn with_extension_mut<T>(
        &self,
        call: impl FnOnce(&mut dyn Extension) -> Result<T>,
    ) -> Result<T> {
        let mut extension = self
            .extension
            .as_ref()
            .ok_or_else(|| anyhow!("source {} is hosted by an extension worker", self.source_id))?
            .write()
            .map_err(|error| {
                anyhow!("source {} write lock is poisoned: {error}", self.source_id)
            })?;
        call(extension.as_mut())
    }

    pub(crate) fn source_name(&self) -> &str {
        &self.source_info.name
    }

    pub(crate) fn plugin_path(&self) -> Option<&Path> {
        self.plugin_path.as_deref()
    }

    pub(crate) fn worker(&self) -> Option<Arc<WorkerClient>> {
        self.worker.clone()
    }

    pub(crate) fn from_worker(
        source_info: WorkerSourceInfo,
        worker: Arc<WorkerClient>,
        max_concurrent_calls: usize,
        plugin_path: PathBuf,
        rustc_version: String,
        lib_version: String,
    ) -> Self {
        let source_info = source_info.into_source_info();
        Self {
            source_id: source_info.id,
            source_info,
            extension: None,
            worker: Some(worker),
            limiter: Arc::new(Semaphore::new(max_concurrent_calls.max(1))),
            health: SourceHealth::new(),
            library: None,
            plugin_path: Some(plugin_path),
            rustc_version,
            lib_version,
        }
    }
}

pub struct Source {
    pub(crate) extension: OnceCell<Box<dyn Extension>>,
    pub(crate) library: Option<LoadedLibrary>,
    pub(crate) plugin_path: Option<PathBuf>,
    pub rustc_version: String,
    pub lib_version: String,
}

impl Source {
    pub fn new(lib: Library, rustc_version: &str, lib_version: &str) -> Source {
        Source {
            library: Some(LoadedLibrary::new(lib)),
            plugin_path: None,
            rustc_version: rustc_version.to_string(),
            lib_version: lib_version.to_string(),
            extension: OnceCell::new(),
        }
    }

    pub fn from(extension: Box<dyn Extension>) -> Self {
        Self {
            library: None,
            plugin_path: None,
            rustc_version: tanoshi_lib::RUSTC_VERSION.to_string(),
            lib_version: tanoshi_lib::LIB_VERSION.to_string(),
            extension: OnceCell::from(extension),
        }
    }

    pub(crate) fn with_plugin_path(mut self, path: PathBuf) -> Self {
        self.plugin_path = Some(path);
        self
    }

    pub(crate) fn into_entry(self, max_concurrent_calls: usize) -> Result<SourceEntry> {
        let max_concurrent_calls = max_concurrent_calls.max(1);
        let extension = self
            .extension
            .get()
            .ok_or_else(|| anyhow!("extension not initiated"))?;
        let source_info = match catch_unwind(AssertUnwindSafe(|| extension.get_source_info())) {
            Ok(source_info) => source_info,
            Err(payload) => {
                let message = panic_payload_message(&*payload);
                log::error!(
                    "EXTENSION PANIC: source=unknown operation=get_source_info payload={message:?}"
                );
                return Err(anyhow!(
                    "[extension-panicked] get_source_info panicked: {message}"
                ));
            }
        };
        let Self {
            extension,
            library,
            plugin_path,
            rustc_version,
            lib_version,
        } = self;
        let extension = extension
            .into_inner()
            .ok_or_else(|| anyhow!("extension not initiated"))?;
        let source_id = source_info.id;

        Ok(SourceEntry {
            source_id,
            source_info,
            extension: Some(RwLock::new(extension)),
            worker: None,
            limiter: Arc::new(Semaphore::new(max_concurrent_calls)),
            health: SourceHealth::new(),
            library,
            plugin_path,
            rustc_version,
            lib_version,
        })
    }
}

impl tanoshi_lib::extensions::PluginRegistrar for Source {
    fn register_function(&mut self, extension: Box<dyn Extension>) {
        self.extension
            .set(extension)
            .map_err(|_| "extension already initiated")
            .unwrap();
    }
}
