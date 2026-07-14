use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use futures::StreamExt;
use rayon::prelude::*;
use serde::Deserialize;

use tanoshi_lib::prelude::Version;
use tanoshi_vm::extension::ExtensionManager;

use crate::{
    domain::{
        entities::{chapter::Chapter, manga::Manga},
        repositories::{
            chapter::ChapterRepository,
            library::{LibraryRepository, LibraryRepositoryError},
            manga::MangaRepository,
        },
    },
    infrastructure::{domain::repositories::user::UserRepositoryImpl, notification::Notification},
};
use tokio::{
    task::JoinHandle,
    time::{self, Instant},
};

const SOURCE_UPDATE_FAILURE_THRESHOLD: usize = 3;
const UPDATE_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const UPDATE_HTTP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
enum MangaUpdateOutcome {
    Success,
    ItemFailure,
    SourceFailure,
}

#[derive(Debug, Default)]
struct SourceUpdateSummary {
    checked: usize,
    succeeded: usize,
    failed: usize,
    skipped: usize,
}

struct SourceUpdateResult {
    summary: SourceUpdateSummary,
    error: Option<anyhow::Error>,
}

fn is_operational_source_failure(error: &anyhow::Error) -> bool {
    const OPERATIONAL_ERROR_PREFIXES: &[&str] = &[
        "[extension-admission]",
        "[extension-circuit-open]",
        "[extension-panicked]",
        "[extension-quarantined]",
        "[extension-saturated]",
        "[extension-timeout]",
        "[extension-worker-busy]",
        "[extension-worker-crashed]",
        "[extension-worker-protocol]",
        "[extension-worker-supervisor]",
        "[extension-worker-timeout]",
    ];

    let message = error.to_string();
    OPERATIONAL_ERROR_PREFIXES
        .iter()
        .any(|prefix| message.starts_with(prefix))
        || error
            .chain()
            .any(|cause| cause.to_string() == "no such source")
        || error.chain().any(|cause| {
            cause
                .downcast_ref::<reqwest::Error>()
                .is_some_and(|error| {
                    error.is_connect()
                        || error.is_request()
                        || error.is_status()
                        || error.is_timeout()
                })
        })
}

#[derive(Debug, Clone)]
pub struct ChapterUpdate {
    pub manga: Manga,
    pub chapter: Chapter,
    pub users: HashSet<i64>,
}

pub type ChapterUpdateReceiver = tokio::sync::broadcast::Receiver<ChapterUpdate>;
pub type ChapterUpdateSender = tokio::sync::broadcast::Sender<ChapterUpdate>;

pub enum ChapterUpdateCommand {
    All(tokio::sync::oneshot::Sender<Result<(), anyhow::Error>>),
    Manga(i64, tokio::sync::oneshot::Sender<Result<(), anyhow::Error>>),
    Library(i64, tokio::sync::oneshot::Sender<Result<(), anyhow::Error>>),
}

impl Display for ChapterUpdateCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChapterUpdateCommand::All(_) => write!(f, "ChapterUpdateCommand::All"),
            ChapterUpdateCommand::Manga(id, _) => write!(f, "ChapterUpdateCommand::Manga({id})"),
            ChapterUpdateCommand::Library(id, _) => {
                write!(f, "ChapterUpdateCommand::Library({id})")
            }
        }
    }
}

pub type ChapterUpdateCommandReceiver = flume::Receiver<ChapterUpdateCommand>;
pub type ChapterUpdateCommandSender = flume::Sender<ChapterUpdateCommand>;

#[derive(Debug, Clone, Deserialize)]
pub struct SourceInfo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub version: String,
    pub icon: String,
    pub nsfw: bool,
}

/// Forward manga from a library stream into the update-check queue, stopping
/// early if the receiving side is dropped.
async fn forward_manga_stream(
    tx: tokio::sync::mpsc::Sender<Manga>,
    mut manga_stream: impl futures::Stream<Item = Result<Manga, LibraryRepositoryError>> + Unpin,
) {
    while let Some(manga_result) = manga_stream.next().await {
        match manga_result {
            Ok(manga) => {
                if let Err(e) = tx.send(manga).await {
                    error!("error forwarding manga to update channel: {e:?}");
                    break;
                }
            }
            Err(e) => {
                error!("error reading manga from library stream: {e:?}");
            }
        }
    }
}

struct UpdatesWorker<C, M, L>
where
    C: ChapterRepository + 'static,
    M: MangaRepository + 'static,
    L: LibraryRepository + 'static,
{
    period: u64,
    max_concurrent_sources: usize,
    client: reqwest::Client,
    library_repo: L,
    manga_repo: M,
    chapter_repo: C,
    extensions: ExtensionManager,
    notifier: Notification<UserRepositoryImpl>,
    extension_repository: String,
    cache_path: PathBuf,
    broadcast_tx: ChapterUpdateSender,
    command_rx: ChapterUpdateCommandReceiver,
}

impl<C, M, L> UpdatesWorker<C, M, L>
where
    C: ChapterRepository + 'static,
    M: MangaRepository + 'static,
    L: LibraryRepository + 'static,
{
    #[allow(clippy::too_many_arguments)]
    fn new<P: AsRef<Path>>(
        period: u64,
        max_concurrent_sources: usize,
        library_repo: L,
        manga_repo: M,
        chapter_repo: C,
        extensions: ExtensionManager,
        notifier: Notification<UserRepositoryImpl>,
        extension_repository: String,
        broadcast_tx: ChapterUpdateSender,
        cache_path: P,
    ) -> (Self, ChapterUpdateCommandSender) {
        #[cfg(not(debug_assertions))]
        let period = if period > 0 && period < 3600 {
            3600
        } else {
            period
        };
        info!("periodic updates every {period} seconds");
        let max_concurrent_sources = max_concurrent_sources.max(1);
        info!("updating up to {max_concurrent_sources} sources concurrently");
        let client = reqwest::Client::builder()
            .connect_timeout(UPDATE_HTTP_CONNECT_TIMEOUT)
            .timeout(UPDATE_HTTP_TIMEOUT)
            .build()
            .expect("failed to build update worker HTTP client");

        let (command_tx, command_rx) = flume::bounded(0);

        (
            Self {
                period,
                max_concurrent_sources,
                client,
                library_repo,
                manga_repo,
                chapter_repo,
                extensions,
                notifier,
                extension_repository,
                cache_path: PathBuf::new().join(cache_path),
                broadcast_tx,
                command_rx,
            },
            command_tx,
        )
    }

    fn start_chapter_update_queue_all(&self, tx: tokio::sync::mpsc::Sender<Manga>) {
        let library_repo = self.library_repo.clone();

        tokio::spawn(async move {
            forward_manga_stream(tx, library_repo.get_manga_from_all_users_library_stream()).await;
        });
    }

    async fn start_chapter_update_queue_by_manga_id(
        &self,
        tx: tokio::sync::mpsc::Sender<Manga>,
        manga_id: i64,
    ) {
        let manga = self.manga_repo.get_manga_by_id(manga_id).await;
        match manga {
            Ok(manga) => {
                if let Err(e) = tx.send(manga).await {
                    error!("error sending manga {} to update channel: {e:?}", manga_id);
                }
            }
            Err(e) => {
                error!("error getting manga {manga_id} for update: {e:?}");
            }
        }
    }

    fn start_chapter_update_queue_by_user_id(
        &self,
        tx: tokio::sync::mpsc::Sender<Manga>,
        user_id: i64,
    ) {
        let library_repo = self.library_repo.clone();

        tokio::spawn(async move {
            forward_manga_stream(tx, library_repo.get_manga_from_user_library_stream(user_id))
                .await;
        });
    }

    async fn check_chapter_update(
        &self,
        mut rx: tokio::sync::mpsc::Receiver<Manga>,
        run_kind: &str,
    ) -> Result<(), anyhow::Error> {
        let mut mangas_by_source: HashMap<i64, Vec<Manga>> = HashMap::new();
        while let Some(manga) = rx.recv().await {
            mangas_by_source
                .entry(manga.source_id)
                .or_default()
                .push(manga);
        }

        let mut source_updates = futures::stream::iter(mangas_by_source)
            .map(|(source_id, mangas)| async move {
                let result = self.check_source_updates(source_id, mangas).await;
                (source_id, result)
            })
            .buffer_unordered(self.max_concurrent_sources);
        let mut first_error = None;
        let mut source_summaries = Vec::new();

        while let Some((source_id, result)) = source_updates.next().await {
            source_summaries.push((source_id, result.summary));
            if let Some(error) = result.error {
                error!("update scan stopped for source {source_id}: {error}");
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }

        source_summaries.sort_unstable_by_key(|(source_id, _)| *source_id);
        let mut run_summary = SourceUpdateSummary::default();
        for (source_id, summary) in source_summaries {
            info!(
                "UPDATE SOURCE SUMMARY: mode={run_kind} source_id={source_id} checked={} succeeded={} failed={} skipped={}",
                summary.checked, summary.succeeded, summary.failed, summary.skipped
            );
            run_summary.checked += summary.checked;
            run_summary.succeeded += summary.succeeded;
            run_summary.failed += summary.failed;
            run_summary.skipped += summary.skipped;
        }
        info!(
            "UPDATE RUN SUMMARY: mode={run_kind} checked={} succeeded={} failed={} skipped={}",
            run_summary.checked,
            run_summary.succeeded,
            run_summary.failed,
            run_summary.skipped
        );

        if let Some(error) = first_error {
            return Err(error);
        }

        Ok(())
    }

    async fn check_source_updates(
        &self,
        source_id: i64,
        mangas: Vec<Manga>,
    ) -> SourceUpdateResult {
        let manga_count = mangas.len();
        let mut consecutive_failures = 0;
        let mut summary = SourceUpdateSummary::default();

        for (index, manga) in mangas.into_iter().enumerate() {
            summary.checked += 1;
            let outcome = match self.check_manga_update(manga).await {
                Ok(outcome) => outcome,
                Err(error) => {
                    summary.failed += 1;
                    summary.skipped = manga_count.saturating_sub(index + 1);
                    return SourceUpdateResult {
                        summary,
                        error: Some(error),
                    };
                }
            };
            match outcome {
                MangaUpdateOutcome::Success => {
                    summary.succeeded += 1;
                    consecutive_failures = 0;
                }
                MangaUpdateOutcome::ItemFailure => {
                    summary.failed += 1;
                    consecutive_failures = 0;
                }
                MangaUpdateOutcome::SourceFailure => {
                    summary.failed += 1;
                    consecutive_failures += 1;
                    if consecutive_failures >= SOURCE_UPDATE_FAILURE_THRESHOLD {
                        let skipped = manga_count.saturating_sub(index + 1);
                        summary.skipped = skipped;
                        error!(
                            "UPDATE SOURCE CIRCUIT OPEN: source_id={source_id} consecutive_operational_failures={consecutive_failures}; skipping {skipped} remaining manga for this run"
                        );
                        break;
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        SourceUpdateResult {
            summary,
            error: None,
        }
    }

    async fn check_manga_update(&self, manga: Manga) -> Result<MangaUpdateOutcome, anyhow::Error> {
        debug!("Checking updates: {}", manga.title);

        let chapters: Vec<Chapter> = match self
            .extensions
            .get_chapters(manga.source_id, manga.path.clone())
            .await
        {
            Ok(chapters) => chapters
                .into_par_iter()
                .map(|ch| {
                    let mut c: Chapter = ch.into();
                    c.manga_id = manga.id;
                    c
                })
                .collect(),
            Err(e) => {
                error!("error fetch new chapters for {}, source {}, reason: {e}", manga.title, manga.source_id);
                return Ok(if is_operational_source_failure(&e) {
                    MangaUpdateOutcome::SourceFailure
                } else {
                    MangaUpdateOutcome::ItemFailure
                });
            }
        };

        self.chapter_repo.insert_chapters(&chapters).await?;

        let chapter_paths: Vec<String> = chapters.into_par_iter().map(|c| c.path).collect();

        if !chapter_paths.is_empty() {
            let chapters_to_delete: Vec<i64> = self
                .chapter_repo
                .get_chapters_not_in_source(manga.source_id, manga.id, &chapter_paths)
                .await?
                .iter()
                .map(|c| c.id)
                .collect();

            if !chapters_to_delete.is_empty() {
                self.chapter_repo
                    .delete_chapter_by_ids(&chapters_to_delete)
                    .await?;
            }
        }

        let last_uploaded_chapter = manga.last_uploaded_at.unwrap_or_default();

        let chapters: Vec<Chapter> = self
            .chapter_repo
            .get_chapters_by_manga_id(manga.id, None, None, false)
            .await?
            .into_par_iter()
            .filter(|chapter| chapter.uploaded > last_uploaded_chapter)
            .collect();

        if chapters.is_empty() {
            debug!("{} has no new chapters", manga.title);
        } else {
            info!("{} has {} new chapters", manga.title, chapters.len());
        }

        let users = self
            .library_repo
            .get_users_by_manga_id(manga.id)
            .await
            .unwrap_or_default();

        for chapter in chapters {
            for user in &users {
                if let Err(error) = self
                    .notifier
                    .send_chapter_notification(
                        user.id,
                        &manga.title,
                        &chapter.title,
                        chapter.id,
                    )
                    .await
                {
                    error!(
                        "failed to notify user {} about chapter '{}' of '{}': {error}",
                        user.id, chapter.title, manga.title
                    );
                }
            }

            if let Err(e) = self.broadcast_tx.send(ChapterUpdate {
                manga: manga.clone(),
                chapter,
                users: users.iter().map(|user| user.id).collect(),
            }) {
                // the send error hands the unsent update back
                error!(
                    "error broadcasting new chapter '{}' for manga '{}': {e}",
                    e.0.chapter.title, manga.title
                );
            }
        }

        Ok(MangaUpdateOutcome::Success)
    }

    async fn check_extension_update(&self) -> Result<(), anyhow::Error> {
        let url = format!("{}/index.json", self.extension_repository);

        let available_sources_map = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<Vec<SourceInfo>>()
            .await?
            .into_par_iter()
            .map(|source| (source.id, source))
            .collect::<HashMap<i64, SourceInfo>>();

        let installed_sources = self.extensions.list().await?;

        for source in installed_sources {
            if available_sources_map
                .get(&source.id)
                .and_then(|index| Version::from_str(&index.version).ok())
                .is_some_and(|v| v > Version::from_str(source.version).unwrap_or_default())
            {
                let message = format!("{} extension update available", source.name);
                if let Err(e) = self.notifier.send_all_to_admins(None, &message).await {
                    error!("failed to send extension update to admin, {e}");
                }
            }
        }

        Ok(())
    }

    async fn check_server_update(&self) -> Result<(), anyhow::Error> {
        #[derive(Debug, Deserialize)]
        struct Release {
            pub tag_name: String,
            pub body: String,
        }

        let release: Release = self
            .client
            .get("https://api.github.com/repos/luigi311/tanoshi/releases/latest")
            .header(
                "User-Agent",
                format!("Tanoshi/{}", env!("CARGO_PKG_VERSION")).as_str(),
            )
            .send()
            .await?
            .json()
            .await?;

        if Version::from_str(&release.tag_name[1..])?
            > Version::from_str(env!("CARGO_PKG_VERSION"))?
        {
            info!("new server update found!");
            let message = format!("Tanoshi {} Released\n{}", release.tag_name, release.body);
            if let Err(e) = self.notifier.send_all_to_admins(None, &message).await {
                error!("failed to send server update notification to admin, {e}");
            }
        } else {
            info!("no tanoshi update found");
        }

        Ok(())
    }

    async fn clear_cache(&self) -> Result<(), anyhow::Error> {
        let mut read_dir = tokio::fs::read_dir(&self.cache_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let meta = match entry.metadata().await {
                Ok(meta) => meta,
                Err(e) => {
                    error!("failed to read metadata of {}: {e}", entry.path().display());
                    continue;
                }
            };

            let age = meta
                .created()
                .ok()
                .and_then(|created| created.elapsed().ok())
                .map(|elapsed| {
                    chrono::Duration::from_std(elapsed).unwrap_or(chrono::Duration::MAX)
                });

            if age.is_some_and(|age| age.num_days() >= 10) {
                info!("removing {}", entry.path().display());
                if let Err(e) = tokio::fs::remove_file(entry.path()).await {
                    error!("failed to remove {}: {e}", entry.path().display());
                }
            }
        }

        Ok(())
    }

    async fn run(self) {
        let period = if self.period == 0 { 3600 } else { self.period };
        let mut chapter_update_interval = time::interval(time::Duration::from_secs(period));
        let mut server_update_interval = time::interval(time::Duration::from_secs(86400));
        let mut clear_cache_interval = time::interval(time::Duration::from_secs(3 * 86400));

        loop {
            tokio::select! {
                Ok(cmd) = self.command_rx.recv_async() => {
                    info!("received command: {cmd}");
                    let (manga_tx, manga_rx) = tokio::sync::mpsc::channel(1);
                    match cmd {
                        ChapterUpdateCommand::All(tx) => {
                            self.start_chapter_update_queue_all(manga_tx);
                            let res = self.check_chapter_update(manga_rx, "manual-all").await;
                            if tx.send(res).is_err() {
                                debug!("chapter update result receiver dropped (All)");
                            }
                        },
                        ChapterUpdateCommand::Manga(manga_id, tx) => {
                            self.start_chapter_update_queue_by_manga_id(manga_tx, manga_id).await;
                            let run_kind = format!("manual-manga:{manga_id}");
                            let res = self.check_chapter_update(manga_rx, &run_kind).await;
                            if tx.send(res).is_err() {
                                debug!("chapter update result receiver dropped (Manga {manga_id})");
                            }
                        },
                        ChapterUpdateCommand::Library(user_id, tx) => {
                            self.start_chapter_update_queue_by_user_id(manga_tx, user_id);
                            let run_kind = format!("manual-library:{user_id}");
                            let res = self.check_chapter_update(manga_rx, &run_kind).await;
                            if tx.send(res).is_err() {
                                debug!("chapter update result receiver dropped (Library user {user_id})");
                            }
                        }
                    }
                }
                start = chapter_update_interval.tick() => {
                    if self.period == 0 {
                        continue;
                    }

                    info!("start periodic updates");

                    let (manga_tx, manga_rx) = tokio::sync::mpsc::channel(1);
                    self.start_chapter_update_queue_all(manga_tx);
                    
                    let check_chapter_result =
                        self.check_chapter_update(manga_rx, "periodic").await;
                    if let Err(e) = check_chapter_result {
                        error!("failed check chapter update: {e}");
                    }

                    info!("periodic updates done in {:?}", Instant::now() - start);
                }
                _ = server_update_interval.tick() => {
                    debug!("check server update");

                    let check_server_result = self.check_server_update().await;
                    if let Err(e) = check_server_result {
                        error!("failed check server update: {e}");
                    }

                    debug!("check extension update");

                    let check_extension_result = self.check_extension_update().await;
                    if let Err(e) = check_extension_result {
                        error!("failed check extension update: {e}");
                    }
                }
                _ = clear_cache_interval.tick() => {
                    let clear_result = self.clear_cache().await;
                    if let Err(e) = clear_result {
                        error!("failed clear cache: {e}");
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn start<C, M, L, P>(
    period: u64,
    max_concurrent_sources: usize,
    library_repo: L,
    manga_repo: M,
    chapter_repo: C,
    extensions: ExtensionManager,
    notifier: Notification<UserRepositoryImpl>,
    extension_repository: String,
    cache_path: P,
) -> (
    ChapterUpdateReceiver,
    ChapterUpdateCommandSender,
    JoinHandle<()>,
)
where
    C: ChapterRepository + 'static,
    M: MangaRepository + 'static,
    L: LibraryRepository + 'static,
    P: AsRef<Path>,
{
    let (broadcast_tx, broadcast_rx) = tokio::sync::broadcast::channel(10);
    let (worker, command_tx) = UpdatesWorker::new(
        period,
        max_concurrent_sources,
        library_repo,
        manga_repo,
        chapter_repo,
        extensions,
        notifier,
        extension_repository,
        broadcast_tx,
        cache_path,
    );

    let handle = tokio::spawn(worker.run());

    (broadcast_rx, command_tx, handle)
}
