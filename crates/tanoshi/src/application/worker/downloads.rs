use crate::{
    domain::{
        entities::{
            chapter::Chapter,
            download::{DownloadQueue, DownloadQueueEntry},
            manga::Manga,
        },
        repositories::{
            chapter::ChapterRepository, download::DownloadRepository, library::LibraryRepository,
            manga::MangaRepository,
        },
    },
    infrastructure::{
        domain::repositories::user::UserRepositoryImpl, local::LocalMangaInfo,
        notification::Notification,
    },
};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use chrono::Utc;
use reqwest::Url;
use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};
use tanoshi_vm::extension::{ExtensionManager, RequestPriority};
use zip::{ZipArchive, ZipWriter, result::ZipError, write::SimpleFileOptions};

use tokio::{
    fs as async_fs,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::{JoinHandle, spawn_blocking},
    time::{Duration, Instant, sleep, sleep_until},
};

use super::updates::ChapterUpdateReceiver;

#[cfg(test)]
#[path = "downloads_tests.rs"]
mod tests;

pub type DownloadSender = UnboundedSender<Command>;
type DownloadReceiver = UnboundedReceiver<Command>;

const MAX_RETRIES: usize = 3;
const RETRY_DELAY_SECS: u64 = 3;
const QUEUE_RETRY_DELAY: Duration = Duration::from_secs(30);

/// Strip characters that are invalid in file names on common filesystems.
fn sanitize_filename(name: &str) -> String {
    name.replace(&['\\', '/', ':', '*', '?', '\"', '<', '>', '|'][..], "")
}

#[derive(Debug)]
pub enum Command {
    InsertIntoQueue(i64),
    InsertIntoQueueBySourcePath(i64, String),
    CleanupCancelledChapter(DownloadQueueEntry),
    Download,
}

pub struct DownloadWorker<C, D, M, L>
where
    C: ChapterRepository + 'static,
    D: DownloadRepository + 'static,
    M: MangaRepository + 'static,
    L: LibraryRepository + 'static,
{
    download_dir: PathBuf,
    chapter_repo: C,
    manga_repo: M,
    download_repo: D,
    library_repo: L,
    ext: ExtensionManager,
    _notifier: Notification<UserRepositoryImpl>,
    rx: DownloadReceiver,
    pending_commands: VecDeque<Command>,
    chapter_update_receiver: ChapterUpdateReceiver,
    auto_download_chapter: bool,
    validated_chapter: Option<i64>,
}

impl<C, D, M, L> DownloadWorker<C, D, M, L>
where
    C: ChapterRepository + 'static,
    D: DownloadRepository + 'static,
    M: MangaRepository + 'static,
    L: LibraryRepository + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new<P: AsRef<Path>>(
        dir: P,
        chapter_repo: C,
        manga_repo: M,
        download_repo: D,
        library_repo: L,
        ext: ExtensionManager,
        notifier: Notification<UserRepositoryImpl>,
        download_receiver: DownloadReceiver,
        chapter_update_receiver: ChapterUpdateReceiver,
        auto_download_chapter: bool,
    ) -> Self {
        Self {
            download_dir: PathBuf::new().join(dir),
            chapter_repo,
            manga_repo,
            download_repo,
            library_repo,
            ext: ext.with_priority(RequestPriority::Low),
            _notifier: notifier,
            rx: download_receiver,
            pending_commands: VecDeque::new(),
            chapter_update_receiver,
            auto_download_chapter,
            validated_chapter: None,
        }
    }

    async fn insert_to_queue(&mut self, chapter: &Chapter) -> Result<(), anyhow::Error> {
        // source ids 10000 and greater are reserved for the local source
        if chapter.source_id >= 10000 {
            anyhow::bail!("local source can't be downloaded");
        }

        if self.validated_chapter == Some(chapter.id) {
            self.validated_chapter = None;
        }

        let existing_path = self
            .download_repo
            .get_chapter_downloaded_path(chapter.id)
            .await
            .unwrap_or_default();

        if !existing_path.is_empty() {
            // Remove the old archive file
            let old_archive = Path::new(&existing_path);
            if async_fs::try_exists(old_archive).await.unwrap_or(false) {
                async_fs::remove_file(old_archive).await?;
            }

            // Clear the downloaded path in the DB
            self.download_repo
                .update_chapter_downloaded_path(chapter.id, None)
                .await?;
        }

        // Also clear any stale queue entries for this chapter
        self.download_repo
            .delete_single_chapter_download_queue(chapter.id)
            .await
            .ok(); // ignore if nothing to delete

        let priority = self
            .download_repo
            .get_download_queue_last_priority()
            .await?
            .map_or(0, |p| p + 1);

        let manga = self.manga_repo.get_manga_by_id(chapter.manga_id).await?;
        let pages = self
            .ext
            .get_pages(manga.source_id, chapter.path.clone())
            .await?;

        let source = self.ext.get_source_info(manga.source_id)?;
        let source_name = sanitize_filename(&source.name);
        let manga_title = sanitize_filename(&manga.title);
        let chapter_title = sanitize_filename(&format!("{} - {}", chapter.number, chapter.title));

        let manga_path = self.download_dir.join(&source_name).join(&manga_title);

        self.save_manga_info_if_not_exists(&manga_path, &manga)
            .await?;

        // Remove any leftover temp file from a previous interrupted download
        let temp_archive = manga_path.join(format!("{}.temp.cbz", chapter_title));
        if async_fs::try_exists(&temp_archive).await.unwrap_or(false) {
            debug!("removing leftover temp archive {}", temp_archive.display());
            async_fs::remove_file(temp_archive).await?;
        }

        let mut queue = vec![];
        let date_added = Utc::now().naive_utc();
        for (rank, page) in pages.iter().enumerate() {
            queue.push(DownloadQueue {
                id: 0,
                source_id: source.id,
                source_name: source_name.clone(),
                manga_id: manga.id,
                manga_title: manga_title.clone(),
                chapter_id: chapter.id,
                chapter_title: chapter_title.clone(),
                rank: rank as _,
                url: page.clone(),
                priority,
                date_added,
            });
        }

        self.download_repo.insert_download_queue(&queue).await?;

        Ok(())
    }

    async fn paused(&self) -> bool {
        async_fs::try_exists(self.download_dir.join(".pause"))
            .await
            .unwrap_or(false)
    }

    async fn cleanup_cancelled_chapter(&mut self, chapter: &DownloadQueueEntry) -> Result<()> {
        // A requeue may have arrived before this cleanup command. Its archive
        // belongs to the new download and must be preserved.
        if !self
            .download_repo
            .get_download_queue(&[chapter.chapter_id])
            .await?
            .is_empty()
        {
            return Ok(());
        }
        if self.validated_chapter == Some(chapter.chapter_id) {
            self.validated_chapter = None;
        }
        let tmp = self
            .download_dir
            .join(&chapter.source_name)
            .join(&chapter.manga_title)
            .join(format!("{}.temp.cbz", chapter.chapter_title));
        match async_fs::remove_file(tmp).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    async fn open_archive(path: &Path, validate: bool) -> Result<Option<ZipArchive<File>>> {
        let path = path.to_owned();
        spawn_blocking(move || {
            match File::open(path) {
                Ok(file) => {
                    let mut archive = ZipArchive::new(file)?;
                    if validate {
                        // Reading through EOF checks both decompression and CRC; a
                        // readable directory alone does not guarantee intact pages.
                        for index in 0..archive.len() {
                            let mut page = archive.by_index(index)?;
                            if let Err(error) = std::io::copy(&mut page, &mut std::io::sink()) {
                                if matches!(
                                    error.kind(),
                                    ErrorKind::InvalidData
                                        | ErrorKind::InvalidInput
                                        | ErrorKind::UnexpectedEof
                                ) {
                                    return Err(ZipError::InvalidArchive(
                                        format!("damaged page {}: {error}", page.name()).into(),
                                    )
                                    .into());
                                }
                                return Err(error.into());
                            }
                        }
                    }
                    Ok(Some(archive))
                }
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.into()),
            }
        })
        .await?
    }

    async fn restart_chapter(&self, queue: &DownloadQueue, tmp: &Path) -> Result<()> {
        warn!(
            "restarting interrupted download for chapter {} of {}",
            queue.chapter_title, queue.manga_title
        );
        // Reset the database first so another interruption cannot leave completed
        // pages pointing at an archive that has already been removed.
        self.download_repo
            .reset_chapter_download_progress(queue.chapter_id)
            .await?;
        match async_fs::remove_file(tmp).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    async fn finish_chapter(
        &self,
        queue: &DownloadQueue,
        tmp: &Path,
        archive_path: &Path,
    ) -> Result<()> {
        if async_fs::try_exists(tmp).await.unwrap_or(false) {
            async_fs::rename(tmp, archive_path).await?;
        }
        self.download_repo
            .update_chapter_downloaded_path(
                queue.chapter_id,
                Some(archive_path.display().to_string()),
            )
            .await?;
        self.download_repo
            .delete_single_chapter_download_queue(queue.chapter_id)
            .await?;
        Ok(())
    }

    async fn save_manga_info_if_not_exists(
        &self,
        manga_path: &PathBuf,
        manga: &Manga,
    ) -> Result<()> {
        let manga_path = manga_path.clone();
        let manga = manga.clone();
        spawn_blocking(move || {
            let path = manga_path.join("details.json");
            if path.exists() {
                return Ok(());
            }

            debug!("creating directory: {}", path.display());
            fs::create_dir_all(manga_path)?;

            let manga_info = LocalMangaInfo {
                title: Some(manga.title),
                author: if manga.author.is_empty() {
                    None
                } else {
                    Some(manga.author)
                },
                genre: Some(manga.genre),
                status: manga.status,
                description: manga.description,
                cover_path: None,
            };

            let mut file = File::create(&path)?;
            serde_json::to_writer_pretty(&mut file, &manga_info)?;

            Ok(())
        })
        .await?
    }

    async fn write_page(
        manga_path: &Path,
        tmp: &Path,
        filename: &str,
        data: Bytes,
        append: bool,
    ) -> Result<()> {
        let manga_path = manga_path.to_owned();
        let tmp = tmp.to_owned();
        let filename = filename.to_owned();
        spawn_blocking(move || {
            #[cfg(test)]
            tests::before_archive_write(&tmp);
            fs::create_dir_all(manga_path)?;
            let mut zip = if append {
                ZipWriter::new_append(fs::OpenOptions::new().read(true).write(true).open(tmp)?)?
            } else {
                ZipWriter::new(File::create(tmp)?)
            };
            zip.start_file(
                filename,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )?;
            zip.write_all(&data)?;
            zip.finish()?.sync_all()?;
            Ok(())
        })
        .await?
    }

    async fn fetch_image(&mut self, queue: &DownloadQueue, url: &Url) -> Result<Option<Bytes>> {
        let ext = self.ext.clone();
        let cancelled = {
            let request = async {
                let mut attempts = 0;
                loop {
                    match ext.get_image_bytes(queue.source_id, url.to_string()).await {
                        Ok(bytes) => return Ok(bytes),
                        Err(error) => {
                            error!(
                                "failed to download {} (attempt {}/{MAX_RETRIES}), reason: {error}",
                                queue.url,
                                attempts + 1
                            );
                        }
                    }
                    attempts += 1;
                    if attempts >= MAX_RETRIES {
                        return Err(anyhow!(
                            "failed to download {url} after {MAX_RETRIES} attempts"
                        ));
                    }
                    sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
                }
            };
            tokio::pin!(request);
            let mut commands_open = true;
            loop {
                tokio::select! {
                    biased;
                    command = self.rx.recv(), if commands_open => {
                        match command {
                            Some(Command::CleanupCancelledChapter(chapter)) => {
                                if chapter.chapter_id == queue.chapter_id {
                                    // A requeue makes the old cleanup stale. Check
                                    // before dropping the current image request.
                                    match self.download_repo.get_download_queue(&[chapter.chapter_id]).await {
                                        Ok(queued) if queued.is_empty() => break chapter,
                                        Ok(_) => continue,
                                        Err(error) => {
                                            error!("failed to check cancellation for chapter {}: {error}", chapter.chapter_id);
                                            continue;
                                        }
                                    }
                                }
                                // No archive write is running while fetching an
                                // image, so other chapters can be cleaned up too.
                                if let Err(error) = self.cleanup_cancelled_chapter(&chapter).await {
                                    error!("failed to clean up cancelled chapter {}: {error}", chapter.chapter_id);
                                }
                            }
                            Some(command) => self.pending_commands.push_back(command),
                            None => commands_open = false,
                        }
                    }
                    result = &mut request => return result.map(Some),
                }
            }
        };
        // Drop the request before cleanup. Pending admission is withdrawn;
        // an already dispatched call remains supervised by the VM and cannot
        // write an archive after this download returns.
        if let Err(error) = self.cleanup_cancelled_chapter(&cancelled).await {
            error!(
                "failed to clean up cancelled chapter {}: {error}",
                cancelled.chapter_id
            );
        }
        Ok(None)
    }

    // Returns whether there was work, including recovery or finalization.
    async fn download(&mut self) -> Result<bool> {
        // Reuse validation only across successful downloads. A failed write
        // may leave damaged entries, so every error forces a fresh check.
        let validated_chapter = self.validated_chapter.take();
        let Some(queue) = self.download_repo.get_single_download_queue().await? else {
            return Ok(false);
        };
        let progress = self
            .download_repo
            .get_download_queue(&[queue.chapter_id])
            .await?;
        let Some(progress) = progress.first() else {
            // The chapter may have been removed while we were reading it.
            return Ok(true);
        };
        let complete = progress.downloaded == progress.total;
        let manga_path = self
            .download_dir
            .join(&queue.source_name)
            .join(&queue.manga_title);
        let archive_path = manga_path.join(format!("{}.cbz", queue.chapter_title));
        let tmp = manga_path.join(format!("{}.temp.cbz", queue.chapter_title));

        // A completed queue may have been interrupted before or after the rename.
        // Never use an older final archive to resume a partially downloaded chapter.
        let archive =
            match Self::open_archive(&tmp, validated_chapter != Some(queue.chapter_id)).await {
                Ok(None) if complete => Self::open_archive(&archive_path, true).await,
                result => result,
            };
        let archive = match archive {
            Ok(archive) => archive,
            Err(error)
                if matches!(
                    error.downcast_ref::<ZipError>(),
                    Some(ZipError::InvalidArchive(_))
                ) =>
            {
                self.restart_chapter(&queue, &tmp).await?;
                return Ok(true);
            }
            Err(error) => return Err(error),
        };
        if archive.as_ref().map_or(0, |archive| archive.len()) < progress.downloaded as usize {
            drop(archive);
            self.restart_chapter(&queue, &tmp).await?;
            return Ok(true);
        }
        if complete {
            drop(archive);
            self.finish_chapter(&queue, &tmp, &archive_path).await?;
            return Ok(true);
        }

        let url = Url::parse(&queue.url)?;
        let filename = format!(
            "{:04}_{}",
            queue.rank,
            url.path_segments()
                .and_then(Iterator::last)
                .ok_or_else(|| anyhow!("no filename"))?
        );
        // If the app stopped after writing the page but before updating SQLite,
        // keep the existing entry instead of appending a duplicate.
        let already_written = archive
            .as_ref()
            .is_some_and(|archive| archive.file_names().any(|name| name == filename));
        let append = archive.is_some();
        drop(archive);
        if !already_written {
            let Some(data) = self.fetch_image(&queue, &url).await? else {
                return Ok(true);
            };

            // Finish and flush in one blocking task, awaiting it before progress
            // updates or cleanup commands can run. A hard exit during this write
            // is handled by archive recovery on the next run.
            Self::write_page(&manga_path, &tmp, &filename, data, append).await?;
        }

        self.download_repo
            .mark_single_download_queue_as_completed(queue.id)
            .await?;
        if self
            .download_repo
            .get_single_chapter_download_status(queue.chapter_id)
            .await?
        {
            self.finish_chapter(&queue, &tmp, &archive_path).await?;
        } else {
            self.validated_chapter = Some(queue.chapter_id);
        }
        Ok(true)
    }

    pub async fn run(mut self) {
        let mut next_download = Some(Instant::now());
        loop {
            // Drain commands received during the fetch before starting more work.
            tokio::select! {
                _ = async {
                    match next_download {
                        Some(deadline) => sleep_until(deadline).await,
                        None => std::future::pending().await,
                    }
                }, if self.pending_commands.is_empty() => {
                    next_download = if self.paused().await {
                        None
                    } else {
                        match self.download().await {
                            Ok(true) => Some(Instant::now()),
                            Ok(false) => None,
                            Err(error) => {
                                error!("download worker error: {error}; retrying in {} seconds", QUEUE_RETRY_DELAY.as_secs());
                                Some(Instant::now() + QUEUE_RETRY_DELAY)
                            }
                        }
                    };
                }
                Ok(chapter) = self.chapter_update_receiver.recv(), if self.pending_commands.is_empty() => {
                    if self.auto_download_chapter {
                        let manga = self.manga_repo.get_manga_by_id(chapter.chapter.manga_id).await;
                        let manga_title = manga.map(|m| m.title).unwrap_or_default();
                        // Check if chapter is already downloaded
                        match self.download_repo.get_chapter_downloaded_path(chapter.chapter.id).await {
                            Ok(path) => {
                                if !path.is_empty() {
                                    debug!("chapter {} for manga {} already downloaded, skipping", chapter.chapter.title, manga_title);
                                    continue;
                                }
                            }
                            Err(e) => {
                                error!("failed to get downloaded path for chapter {}, reason: {:?}", chapter.chapter.id, e);
                            }
                        }
                        match self.library_repo.get_users_by_manga_id(chapter.chapter.manga_id).await {
                            Ok(users) => {
                                if users.is_empty() {
                                    debug!("manga {} not in library, skipping auto download for chapter {}", manga_title, chapter.chapter.title);
                                    continue;
                                }
                            }
                            Err(e) => {
                                error!("failed to get library users for manga {}, reason: {:?}", manga_title, e);
                            }
                        }

                        let insert_result = self
                            .insert_to_queue(&chapter.chapter)
                            .await;
                        match insert_result {
                            Err(e) => {
                                error!("failed to insert queue, reason {e}");
                            } Ok(()) => {
                                next_download.get_or_insert_with(Instant::now);
                            }
                        }
                    }
                }
                Some(cmd) = async {
                    match self.pending_commands.pop_front() {
                        Some(command) => Some(command),
                        None => self.rx.recv().await,
                    }
                } => {
                    match cmd {
                        Command::InsertIntoQueue(chapter_id) => {
                            let chapter_result = self
                                .chapter_repo
                                .get_chapter_by_id(chapter_id)
                                .await;
                            match chapter_result {
                                Ok(chapter) => {
                                    let insert_result = self.insert_to_queue(&chapter).await;
                                    match insert_result {
                                        Err(e) => {
                                            error!("failed to insert queue, reason {e}");
                                        }
                                        Ok(()) => {
                                            next_download.get_or_insert_with(Instant::now);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("chapter {chapter_id} not found, {e}");
                                }
                            }
                        }
                        Command::InsertIntoQueueBySourcePath(source_id, path) => {
                            let chapter_result = self
                                .chapter_repo
                                .get_chapter_by_source_id_path(source_id, &path)
                                .await;
                            match chapter_result {
                                Ok(chapter) => {
                                    let insert_result = self.insert_to_queue(&chapter).await;
                                    match insert_result {
                                        Err(e) => {
                                            error!("failed to insert queue, reason {e}");
                                        }
                                        Ok(()) => {
                                            next_download.get_or_insert_with(Instant::now);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("chapter {source_id} {path} not found: {e}");
                                }
                            }
                        }
                        Command::CleanupCancelledChapter(chapter) => {
                            // Commands run between page downloads, so an image
                            // request cannot recreate the archive after cleanup.
                            if let Err(error) = self.cleanup_cancelled_chapter(&chapter).await {
                                error!("failed to clean up cancelled chapter {}: {error}", chapter.chapter_id);
                            }
                        }
                        Command::Download => {
                            next_download = Some(Instant::now());
                        }
                    }
                }
            }
        }
    }
}

pub fn channel() -> (DownloadSender, DownloadReceiver) {
    tokio::sync::mpsc::unbounded_channel::<Command>()
}

#[allow(clippy::too_many_arguments)]
pub fn start<C, D, M, L, P>(
    dir: P,
    chapter_repo: C,
    manga_repo: M,
    download_repo: D,
    library_repo: L,
    ext: ExtensionManager,
    notifier: Notification<UserRepositoryImpl>,
    download_receiver: DownloadReceiver,
    chapter_update_receiver: ChapterUpdateReceiver,
    auto_download_chapter: bool,
) -> JoinHandle<()>
where
    C: ChapterRepository + 'static,
    D: DownloadRepository + 'static,
    M: MangaRepository + 'static,
    L: LibraryRepository + 'static,
    P: AsRef<Path>,
{
    let download_worker = DownloadWorker::new(
        dir,
        chapter_repo,
        manga_repo,
        download_repo,
        library_repo,
        ext,
        notifier,
        download_receiver,
        chapter_update_receiver,
        auto_download_chapter,
    );

    tokio::spawn(download_worker.run())
}
