use std::{
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::sync::atomic::{AtomicBool, Ordering};
use tanoshi_lib::prelude::{ChapterInfo, Input, MangaInfo, PluginDeclaration, SourceInfo};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader as AsyncBufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{Mutex, Notify},
    time::Instant,
};

use super::{Source, SourceEntry, source::panic_payload_message};

const PROTOCOL_VERSION: u32 = 1;
const MAX_FRAME_SIZE: usize = 128 * 1024 * 1024;
const WORKER_BINARY_NAME: &str = "tanoshi-extension-worker";
pub const WORKER_MODE_FLAG: &str = "--tanoshi-extension-worker";

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug)]
pub(crate) enum WorkerCallError {
    /// The worker held the request past the deadline and was terminated.
    Timeout,
    /// The deadline expired while earlier calls still occupied the worker
    /// stream; the worker itself was left untouched.
    QueueTimeout,
    /// The client was explicitly shut down because its source was unloaded
    /// or replaced.
    Stopped,
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
            Self::Crashed(message) => write!(formatter, "extension worker exited: {message}"),
            Self::Remote { kind, message } => {
                write!(formatter, "extension worker returned {kind:?}: {message}")
            }
        }
    }
}

impl std::error::Error for WorkerCallError {}

struct WorkerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: AsyncBufReader<ChildStdout>,
    next_request_id: u64,
    source_info: WorkerSourceInfo,
    rustc_version: String,
    lib_version: String,
}

pub(crate) struct WorkerClient {
    plugin_path: PathBuf,
    worker_path: PathBuf,
    startup_timeout: Duration,
    cleanup_path: Option<PathBuf>,
    stopped: AtomicBool,
    process: Mutex<Option<WorkerProcess>>,
    shutdown: Notify,
    // Saved preferences to re-apply whenever a replacement worker spawns, so
    // a respawned worker never serves requests with default preferences.
    startup_preferences: StdMutex<Option<Vec<Input>>>,
}

impl WorkerClient {
    pub(crate) fn new(
        plugin_path: PathBuf,
        worker_path: PathBuf,
        startup_timeout: Duration,
        cleanup_path: Option<PathBuf>,
    ) -> Arc<Self> {
        Arc::new(Self {
            plugin_path,
            worker_path,
            startup_timeout,
            cleanup_path,
            stopped: AtomicBool::new(false),
            process: Mutex::new(None),
            shutdown: Notify::new(),
            startup_preferences: StdMutex::new(None),
        })
    }

    pub(crate) fn set_startup_preferences(&self, preferences: Vec<Input>) {
        let mut guard = match self.startup_preferences.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = Some(preferences);
    }

    pub(crate) async fn start(&self) -> Result<(WorkerSourceInfo, String, String)> {
        let mut process = self.process.lock().await;
        if self.stopped.load(Ordering::Acquire) {
            bail!("extension worker is shut down");
        }
        if process.is_none() {
            let (worker, source_info, rustc_version, lib_version) =
                tokio::time::timeout(self.startup_timeout, self.spawn_process())
                    .await
                    .context("extension worker startup timed out")??;
            *process = Some(worker);
            return Ok((source_info, rustc_version, lib_version));
        }

        let Some(process) = process.as_mut() else {
            unreachable!("worker process was inserted above");
        };
        Ok((
            process.source_info.clone(),
            process.rustc_version.clone(),
            process.lib_version.clone(),
        ))
    }

    pub(crate) async fn request(
        &self,
        request: WorkerRequest,
        timeout: Duration,
    ) -> std::result::Result<WorkerValue, WorkerCallError> {
        // The deadline covers the wait for the per-source stream as well as
        // the request itself, so a caller never waits longer than its own
        // timeout behind earlier calls.
        let deadline = Instant::now() + timeout;
        // Register interest in shutdown before checking `stopped` so a
        // shutdown between the check and the select cannot be missed.
        let shutdown = self.shutdown.notified();
        tokio::pin!(shutdown);
        shutdown.as_mut().enable();
        if self.stopped.load(Ordering::Acquire) {
            return Err(WorkerCallError::Stopped);
        }

        let mut process_guard = tokio::select! {
            guard = tokio::time::timeout_at(deadline, self.process.lock()) => match guard {
                Ok(guard) => guard,
                Err(_) => return Err(WorkerCallError::QueueTimeout),
            },
            _ = &mut shutdown => return Err(WorkerCallError::Stopped),
        };
        if self.stopped.load(Ordering::Acquire) {
            return Err(WorkerCallError::Stopped);
        }
        if process_guard.is_none() {
            // Respawn after an earlier failure. Repeated failures are bounded
            // by the source's health policy: once the entry is quarantined,
            // admission stops before another spawn is attempted.
            let spawn = tokio::time::timeout_at(deadline, self.spawn_process());
            tokio::pin!(spawn);
            let result = tokio::select! {
                result = &mut spawn => result,
                _ = &mut shutdown => return Err(WorkerCallError::Stopped),
            };
            match result {
                Ok(Ok((worker, _, _, _))) => *process_guard = Some(worker),
                Ok(Err(error)) => return Err(WorkerCallError::Crashed(error.to_string())),
                Err(_) => return Err(WorkerCallError::QueueTimeout),
            }
        }
        if Instant::now() >= deadline {
            // The spawned or running worker stays usable for later callers.
            return Err(WorkerCallError::QueueTimeout);
        }

        let process = process_guard
            .as_mut()
            .expect("worker process was initialized above");
        let id = process.next_request_id;
        process.next_request_id = process.next_request_id.wrapping_add(1);
        let envelope = WorkerRequestEnvelope { id, request };

        let response = tokio::select! {
            response = tokio::time::timeout_at(deadline, async {
                write_frame_async(&mut process.stdin, &envelope).await?;
                read_frame_async::<_, WorkerResponse>(&mut process.stdout).await
            }) => match response {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                terminate_process(process).await;
                *process_guard = None;
                return Err(WorkerCallError::Crashed(error.to_string()));
            }
            Err(_) => {
                terminate_process(process).await;
                *process_guard = None;
                return Err(WorkerCallError::Timeout);
            }
            },
            _ = &mut shutdown => {
                terminate_process(process).await;
                *process_guard = None;
                return Err(WorkerCallError::Stopped);
            }
        };

        match response {
            WorkerResponse::Result {
                id: response_id,
                value,
            } if response_id == id => Ok(value),
            WorkerResponse::Error {
                id: response_id,
                kind,
                message,
            } if response_id == id => Err(WorkerCallError::Remote { kind, message }),
            WorkerResponse::Result {
                id: response_id, ..
            }
            | WorkerResponse::Error {
                id: response_id, ..
            } => {
                terminate_process(process).await;
                *process_guard = None;
                Err(WorkerCallError::Crashed(format!(
                    "response id {response_id} did not match request id {id}"
                )))
            }
            WorkerResponse::Ready { .. } => {
                terminate_process(process).await;
                *process_guard = None;
                Err(WorkerCallError::Crashed(
                    "worker sent an unexpected readiness response".to_string(),
                ))
            }
        }
    }

    pub(crate) async fn pause(&self) {
        self.stopped.store(true, Ordering::Release);
        self.shutdown.notify_waiters();
        let mut process = self.process.lock().await;
        if let Some(process) = process.as_mut() {
            terminate_process(process).await;
        }
        *process = None;
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
                "failed to remove extension worker staging file {}: {error}",
                path.display()
            );
        }
    }

    async fn spawn_process(&self) -> Result<(WorkerProcess, WorkerSourceInfo, String, String)> {
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
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("extension worker stdin was not piped"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("extension worker stdout was not piped"))?;
        let mut worker = WorkerProcess {
            child,
            stdin,
            stdout: AsyncBufReader::new(stdout),
            next_request_id: 1,
            source_info: WorkerSourceInfo {
                id: 0,
                name: String::new(),
                url: String::new(),
                version: String::new(),
                icon: String::new(),
                languages: tanoshi_lib::prelude::Lang::All,
                nsfw: false,
            },
            rustc_version: String::new(),
            lib_version: String::new(),
        };

        let response = read_frame_async::<_, WorkerResponse>(&mut worker.stdout)
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
                terminate_process(&mut worker).await;
                bail!(
                    "extension worker protocol mismatch: worker={protocol_version} host={PROTOCOL_VERSION}"
                );
            }
            other => {
                terminate_process(&mut worker).await;
                bail!("extension worker did not send readiness: {other:?}");
            }
        };
        worker.source_info = source_info.clone();
        worker.rustc_version = rustc_version.clone();
        worker.lib_version = lib_version.clone();

        if let Err(error) = self.apply_startup_preferences(&mut worker).await {
            terminate_process(&mut worker).await;
            return Err(error);
        }

        Ok((worker, source_info, rustc_version, lib_version))
    }

    /// Re-applies the saved preferences to a freshly spawned worker before it
    /// serves any request.
    async fn apply_startup_preferences(&self, worker: &mut WorkerProcess) -> Result<()> {
        let preferences = match self.startup_preferences.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        let Some(preferences) = preferences else {
            return Ok(());
        };

        let id = worker.next_request_id;
        worker.next_request_id = worker.next_request_id.wrapping_add(1);
        let envelope = WorkerRequestEnvelope {
            id,
            request: WorkerRequest::SetPreferences { preferences },
        };
        let response = async {
            write_frame_async(&mut worker.stdin, &envelope).await?;
            read_frame_async::<_, WorkerResponse>(&mut worker.stdout).await
        }
        .await
        .context("failed to apply saved preferences to the extension worker")?;

        match response {
            WorkerResponse::Result {
                id: response_id,
                value: WorkerValue::Unit,
            } if response_id == id => Ok(()),
            WorkerResponse::Error { kind, message, .. } => {
                bail!("extension worker rejected saved preferences ({kind:?}): {message}")
            }
            other => {
                bail!(
                    "extension worker sent an unexpected response to saved preferences: {other:?}"
                )
            }
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

    let entry = load_worker_entry(&plugin_path)?;
    let mut input = BufReader::new(io::stdin().lock());
    let mut output = BufWriter::new(io::stdout().lock());
    write_frame_sync(
        &mut output,
        &WorkerResponse::Ready {
            protocol_version: PROTOCOL_VERSION,
            source_info: WorkerSourceInfo::from(&entry.source_info),
            rustc_version: entry.rustc_version.clone(),
            lib_version: entry.lib_version.clone(),
        },
    )?;

    while let Some(request) = read_frame_sync::<_, WorkerRequestEnvelope>(&mut input)? {
        let response = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute_request(&entry, request.request)
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
        };
        write_frame_sync(&mut output, &response)?;
    }

    Ok(())
}

fn load_worker_entry(plugin_path: &Path) -> Result<Arc<SourceEntry>> {
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

    Ok(Arc::new(source.into_entry(1)?))
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

async fn terminate_process(process: &mut WorkerProcess) {
    let _ = process.child.kill().await;
    let _ = process.child.wait().await;
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
