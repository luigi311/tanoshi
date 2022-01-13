use std::{
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    db::{
        model::{Chapter, DownloadQueue},
        MangaDatabase,
    },
    notifier::Notifier,
};
use reqwest::Url;
use tanoshi_vm::extension::SourceBus;

use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

pub type DownloadSender = UnboundedSender<Command>;
type DownloadReceiver = UnboundedReceiver<Command>;

#[derive(Debug)]
pub enum Command {
    InsertIntoQueue(i64),
    InsertIntoQueueBySourcePath(i64, String),
    Download,
}

pub struct DownloadWorker {
    dir: PathBuf,
    client: reqwest::Client,
    db: MangaDatabase,
    ext: SourceBus,
    _notifier: Notifier,
    tx: DownloadSender,
    rx: DownloadReceiver,
}

impl DownloadWorker {
    pub fn new<P: AsRef<Path>>(
        dir: P,
        db: MangaDatabase,
        ext: SourceBus,
        notifier: Notifier,
    ) -> (Self, DownloadSender) {
        let (tx, rx) = unbounded_channel::<Command>();
        (
            Self {
                dir: PathBuf::new().join(dir),
                client: reqwest::ClientBuilder::new().build().unwrap(),
                db,
                ext,
                _notifier: notifier,
                rx,
                tx: tx.clone(),
            },
            tx,
        )
    }

    async fn insert_to_queue(&mut self, chapter: &Chapter) -> Result<(), anyhow::Error> {
        // numbe 1 and greater than 10000 reserved for local source
        if chapter.source_id >= 10000 {
            anyhow::bail!("local source can't be downloaded");
        }

        let priority = self
            .db
            .get_download_queue_last_priority()
            .await?
            .map(|p| p + 1)
            .unwrap_or(0);

        let manga = self.db.get_manga_by_id(chapter.manga_id).await?;
        let pages = self
            .ext
            .get_pages(manga.source_id, chapter.path.clone())
            .await?;

        let source = self.ext.get_source_info(manga.source_id).await?;

        let mut queue = vec![];
        let date_added = chrono::Utc::now().naive_utc();
        for (rank, page) in pages.iter().enumerate() {
            queue.push(DownloadQueue {
                id: 0,
                source_id: source.id,
                source_name: source.name.to_string(),
                manga_id: manga.id,
                manga_title: manga.title.clone(),
                chapter_id: chapter.id,
                chapter_title: format!("{} - {}", chapter.number, chapter.title.clone()),
                rank: rank as _,
                url: page.clone(),
                priority,
                date_added,
            })
        }
        debug!("queue: {:?}", queue);
        self.db.insert_download_queue(&queue).await?;

        Ok(())
    }

    async fn paused(&self) -> bool {
        self.dir.join(".pause").exists()
    }

    pub async fn run(&mut self) {
        if !self.paused().await {
            self.tx.send(Command::Download).unwrap();
        }

        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                Command::InsertIntoQueue(chapter_id) => {
                    match self.db.get_chapter_by_id(chapter_id).await {
                        Ok(chapter) => {
                            if let Err(e) = self.insert_to_queue(&chapter).await {
                                error!("failed to insert queue, reason {}", e);
                                continue;
                            }
                            self.tx.send(Command::Download).unwrap();
                        }
                        Err(e) => {
                            error!("chapter {} not found, {}", chapter_id, e);
                        }
                    }
                }
                Command::InsertIntoQueueBySourcePath(source_id, path) => {
                    match self.db.get_chapter_by_source_path(source_id, &path).await {
                        Some(chapter) => {
                            if let Err(e) = self.insert_to_queue(&chapter).await {
                                error!("failed to insert queue, reason {}", e);
                                continue;
                            }
                            self.tx.send(Command::Download).unwrap();
                        }
                        None => {
                            error!("chapter {} {} not found", source_id, path);
                        }
                    }
                }
                Command::Download => {
                    if self.paused().await {
                        continue;
                    }

                    let queue = match self.db.get_single_download_queue().await {
                        Ok(Some(queue)) => queue,
                        Err(e) => {
                            error!("error get single download queue: {}", e);
                            continue;
                        }
                        Ok(None) => {
                            info!("download done");
                            continue;
                        }
                    };

                    debug!("got {}", queue.url);

                    let url = if let Ok(url) = Url::parse(&queue.url) {
                        url
                    } else {
                        continue;
                    };

                    let filename = if let Some(filename) = url
                        .path_segments()
                        .and_then(|seg| seg.last())
                        .map(|s| s.to_string())
                    {
                        filename
                    } else {
                        continue;
                    };

                    let source_name = if cfg!(windows) {
                        queue
                            .source_name
                            .replace(&['\\', '/', ':', '*', '?', '\"', '<', '>', '|'][..], "")
                    } else {
                        queue.source_name
                    };

                    let manga_title = if cfg!(windows) {
                        queue
                            .manga_title
                            .replace(&['\\', '/', ':', '*', '?', '\"', '<', '>', '|'][..], "")
                    } else {
                        queue.manga_title
                    };

                    let chapter_title = if cfg!(windows) {
                        queue
                            .chapter_title
                            .replace(&['\\', '/', ':', '*', '?', '\"', '<', '>', '|'][..], "")
                    } else {
                        queue.chapter_title
                    };

                    let manga_path = self.dir.join(&source_name).join(&manga_title);

                    let archive_path = manga_path.join(format!("{}.cbz", chapter_title));

                    if let Ok(file) = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&archive_path)
                    {
                        if zip::ZipArchive::new(&file)
                            .map(|mut zip| zip.by_name(&filename).is_ok())
                            .unwrap_or_default()
                        {
                            debug!("file already downloaded, mark as compeleted then skip");
                            if let Err(e) = self
                                .db
                                .mark_single_download_queue_as_completed(queue.id)
                                .await
                            {
                                error!(
                                    "error mark single download queue as completed, reason {}",
                                    e
                                );
                            }
                            if !self.paused().await {
                                self.tx.send(Command::Download).unwrap();
                            }
                            continue;
                        }
                    }

                    let mut zip = if let Ok(file) = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&archive_path)
                    {
                        match zip::ZipWriter::new_append(file) {
                            Ok(zip) => zip,
                            Err(e) => {
                                error!("failed to open archive file, reason {}", e);
                                continue;
                            }
                        }
                    } else if let Ok(file) = std::fs::create_dir_all(manga_path)
                        .and_then(|_| std::fs::File::create(&archive_path))
                    {
                        zip::ZipWriter::new(file)
                    } else {
                        continue;
                    };

                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    let referrer = self
                        .ext
                        .get_source_info(queue.source_id)
                        .await
                        .map(|s| s.url.to_string())
                        .unwrap_or_default();

                    let res = if let Ok(res) = self
                        .client
                        .request(reqwest::Method::GET, url.clone())
                        .header("referer", referrer)
                        .send()
                        .await
                    {
                        res
                    } else {
                        continue;
                    };

                    let contents = if let Ok(contents) = res.bytes().await {
                        contents
                    } else {
                        continue;
                    };

                    if let Err(e) = zip.start_file(&filename, Default::default()) {
                        error!("failed to create file inside archive, reason {}", e);
                        continue;
                    }

                    match zip.write_all(contents.to_vec().as_slice()) {
                        Ok(_) => {
                            info!("downloaded to {}", archive_path.display());
                        }
                        Err(e) => {
                            error!("error downloading {}, reason {}", url, e);
                            continue;
                        }
                    }

                    if let Err(e) = self
                        .db
                        .mark_single_download_queue_as_completed(queue.id)
                        .await
                    {
                        error!(
                            "error mark single download queue as completed, reason {}",
                            e
                        );
                        continue;
                    }

                    if self
                        .db
                        .get_single_chapter_download_status(queue.chapter_id)
                        .await
                        .unwrap_or_default()
                    {
                        if let Err(e) = self
                            .db
                            .update_chapter_downloaded_path(
                                queue.chapter_id,
                                Some(archive_path.display().to_string()),
                            )
                            .await
                        {
                            error!("error update chapter downloaded path, {}", e);
                            continue;
                        }
                        if let Err(e) = self
                            .db
                            .delete_single_chapter_download_queue(queue.chapter_id)
                            .await
                        {
                            error!("error delete single chapter download queue, {}", e);
                            continue;
                        }
                    }

                    if let Err(e) = zip.finish() {
                        error!("error finishing zip, reason {}", e);
                    }

                    if !self.paused().await {
                        self.tx.send(Command::Download).unwrap();
                    }
                }
            }
        }
    }
}

pub fn start<P: AsRef<Path>>(
    dir: P,
    mangadb: MangaDatabase,
    ext: SourceBus,
    notifier: Notifier,
) -> (DownloadSender, JoinHandle<()>) {
    let (mut download_worker, tx) = DownloadWorker::new(dir, mangadb, ext, notifier);

    let handle = tokio::spawn(async move {
        download_worker.run().await;
    });

    (tx, handle)
}
