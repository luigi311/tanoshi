use std::path::{Path, PathBuf};

use crate::db::MangaDatabase;
use reqwest::Url;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

pub struct DownloadWorker {
    dir: PathBuf,
    client: reqwest::Client,
    db: MangaDatabase,
    rx: UnboundedReceiver<()>,
}

impl DownloadWorker {
    pub fn new<P: AsRef<Path>>(dir: P, db: MangaDatabase) -> (Self, UnboundedSender<()>) {
        let (tx, rx) = unbounded_channel::<()>();
        (
            Self {
                dir: PathBuf::new().join(dir),
                client: reqwest::ClientBuilder::new().build().unwrap(),
                db,
                rx,
            },
            tx,
        )
    }

    pub async fn run(&mut self) {
        loop {
            info!("start downloading...");
            while let Ok(Some(queue)) = self.db.get_single_download_queue().await {
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

                let path = self
                    .dir
                    .join(source_name)
                    .join(manga_title)
                    .join(chapter_title)
                    .join(filename);

                if path.exists() {
                    info!("{} downloaded. continue...", path.display());
                    if self
                        .db
                        .delete_single_download_queue_by_id(queue.id)
                        .await
                        .is_err()
                    {
                        continue;
                    }
                    continue;
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let res = if let Ok(res) = self.client.get(url.clone()).send().await {
                    res
                } else {
                    continue;
                };

                let contents = if let Ok(contents) = res.bytes().await {
                    contents
                } else {
                    continue;
                };

                if let Some(parent) = path.parent() {
                    if let Err(e) = tokio::fs::create_dir_all(parent).await {
                        error!("error create parent directory {}, {}", parent.display(), e);
                    }
                } else {
                    continue;
                }

                match tokio::fs::write(&path, &contents).await {
                    Ok(_) => {
                        info!("downloaded to {}", path.display());
                        if self
                            .db
                            .update_page_by_url(&queue.url, path.display().to_string().as_str())
                            .await
                            .is_err()
                        {
                            continue;
                        }
                        if self
                            .db
                            .delete_single_download_queue_by_id(queue.id)
                            .await
                            .is_err()
                        {
                            continue;
                        }
                    }
                    Err(e) => {
                        error!("error downloading {}, reason {}", url, e);
                    }
                }
            }
            info!("done downloading");

            let _ = self.rx.recv().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

pub fn start<P: AsRef<Path>>(
    dir: P,
    mangadb: MangaDatabase,
) -> (UnboundedSender<()>, JoinHandle<()>) {
    let (mut download_worker, tx) = DownloadWorker::new(dir, mangadb);

    let handle = tokio::spawn(async move {
        download_worker.run().await;
    });

    (tx, handle)
}
