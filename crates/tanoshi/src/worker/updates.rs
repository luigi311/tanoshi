use std::{collections::HashMap, fmt::Display, str::FromStr};

use chrono::Utc;
use serde::Deserialize;
use tanoshi_lib::prelude::Version;

use tanoshi_vm::prelude::ExtensionBus;
use tokio::sync::mpsc::Sender;
use tokio::{
    task::JoinHandle,
    time::{self, Instant},
};

use crate::config::GLOBAL_CONFIG;
use crate::db::model;
use crate::local;
use crate::{
    db::{
        model::{Chapter, User},
        MangaDatabase, UserDatabase,
    },
    worker::Command as WorkerCommand,
};

#[derive(Debug, Clone)]
struct ChapterUpdate {
    manga_title: String,
    cover_url: String,
    title: String,
}

impl Display for ChapterUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let manga_title = html_escape::encode_safe(&self.manga_title).to_string();
        let title = html_escape::encode_safe(&self.title).to_string();

        write!(f, "<b>{}</b>\n{}", manga_title, title)
    }
}

struct UpdatesWorker {
    period: u64,
    client: reqwest::Client,
    userdb: UserDatabase,
    mangadb: MangaDatabase,
    extensions: ExtensionBus,
    auto_download_chapters: bool,
    worker_tx: Sender<WorkerCommand>,
}

impl UpdatesWorker {
    fn new(
        period: u64,
        userdb: UserDatabase,
        mangadb: MangaDatabase,
        extensions: ExtensionBus,
        worker_tx: Sender<WorkerCommand>,
    ) -> Self {
        #[cfg(not(debug_assertions))]
        let period = if period > 0 && period < 3600 {
            3600
        } else {
            period
        };
        info!("periodic updates every {} seconds", period);

        let auto_download_chapters = GLOBAL_CONFIG
            .get()
            .map(|cfg| cfg.auto_download_chapters)
            .unwrap_or(false);

        Self {
            period,
            client: reqwest::Client::new(),
            userdb,
            mangadb,
            extensions,
            auto_download_chapters,
            worker_tx,
        }
    }

    async fn add_chapter_to_download_queue(&self, chapter: &Chapter) -> Result<(), anyhow::Error> {
        let chapter = self
            .mangadb
            .get_chapter_by_source_path(chapter.source_id, &chapter.path)
            .await
            .ok_or_else(|| anyhow::anyhow!("chapter not found"))?;
        if chapter.source_id == local::ID {
            info!("local source can't be downloaded");
            return Ok(());
        }

        let manga = self.mangadb.get_manga_by_id(chapter.manga_id).await?;
        let pages = match self
            .mangadb
            .get_pages_remote_url_by_chapter_id(chapter.id)
            .await
        {
            Ok(pages) => pages,
            Err(_) => {
                let pages = self
                    .extensions
                    .get_pages(manga.source_id, chapter.path.clone())
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                self.mangadb.insert_pages(chapter.id, &pages).await?;
                pages
            }
        };

        let source = self
            .extensions
            .detail(manga.source_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut queue = vec![];
        let date_added = Utc::now().naive_utc();
        for (rank, page) in pages.iter().enumerate() {
            queue.push(model::DownloadQueue {
                id: 0,
                source_id: source.id,
                source_name: source.name.clone(),
                manga_id: manga.id,
                manga_title: manga.title.clone(),
                chapter_id: chapter.id,
                chapter_title: chapter.title.clone(),
                rank: rank as _,
                url: page.clone(),
                date_added,
            })
        }
        debug!("queue: {:?}", queue);
        self.mangadb.insert_download_queue(&queue).await?;

        Ok(())
    }

    async fn check_chapter_update(&self) -> Result<(), anyhow::Error> {
        let manga_in_library = self.mangadb.get_all_user_library().await?;

        let mut user_map: HashMap<i64, User> = HashMap::new();

        for item in manga_in_library {
            let last_uploaded_chapter = self
                .mangadb
                .get_last_uploaded_chapters_by_manga_id(item.manga.id)
                .await
                .map(|ch| ch.uploaded);

            info!("Checking updates: {}", item.manga.title);

            let chapters = match self
                .extensions
                .get_chapters(item.manga.source_id, item.manga.path.clone())
                .await
            {
                Ok(chapters) => {
                    let chapters: Vec<Chapter> = chapters
                        .into_iter()
                        .map(|ch| {
                            let mut c: Chapter = ch.into();
                            c.manga_id = item.manga.id;
                            c
                        })
                        .collect();
                    chapters
                }
                Err(e) => {
                    error!("error fetch new chapters, reason: {}", e);
                    continue;
                }
            };

            if let Err(e) = self.mangadb.insert_chapters(&chapters).await {
                error!("error inserting new chapters, reason: {}", e);
                continue;
            }

            let chapters = if let Some(last_uploaded_chapter) = last_uploaded_chapter {
                chapters
                    .into_iter()
                    .filter(|ch| ch.uploaded > last_uploaded_chapter)
                    .collect()
            } else {
                chapters
            };

            info!("Found: {} new chapters", chapters.len());

            for chapter in chapters {
                for user_id in item.user_ids.iter() {
                    let user = match user_map.get(user_id) {
                        Some(user) => user.to_owned(),
                        None => {
                            let user = self.userdb.get_user_by_id(*user_id).await?;
                            user_map.insert(*user_id, user.to_owned());
                            user
                        }
                    };

                    if let Some(chat_id) = user.telegram_chat_id {
                        let update = ChapterUpdate {
                            manga_title: item.manga.title.clone(),
                            cover_url: item.manga.cover_url.clone(),
                            title: chapter.title.clone(),
                        };
                        if let Err(e) = self
                            .worker_tx
                            .send(WorkerCommand::TelegramMessage(chat_id, update.to_string()))
                            .await
                        {
                            error!("failed to send message, reason: {}", e);
                        }
                    }

                    if let Some(user_key) = user.pushover_user_key {
                        if let Err(e) = self
                            .worker_tx
                            .send(WorkerCommand::PushoverMessage {
                                user_key,
                                title: Some(item.manga.title.clone()),
                                body: chapter.title.clone(),
                            })
                            .await
                        {
                            error!("failed to send PushoverMessage, reason: {}", e);
                        }
                    }
                }

                if self.auto_download_chapters {
                    info!("add chapter to download queue");
                    if let Err(e) = self.add_chapter_to_download_queue(&chapter).await {
                        error!("failed to add chapter to download queue, reason: {}", e);
                        continue;
                    }
                    let _ = self.worker_tx.send(WorkerCommand::StartDownload).await;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Ok(())
    }

    async fn check_extension_update(&self) -> Result<(), anyhow::Error> {
        #[derive(Debug, Clone, Deserialize)]
        pub struct SourceIndex {
            pub id: i64,
            pub name: String,
            pub path: String,
            pub version: String,
            pub icon: String,
        }

        let url = "https://faldez.github.io/tanoshi-extensions".to_string();
        let available_sources_map = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<Vec<SourceIndex>>()
            .await?
            .into_iter()
            .map(|source| (source.id, source))
            .collect::<HashMap<i64, SourceIndex>>();

        let admins = self.userdb.get_admins().await?;
        let installed_sources = self
            .extensions
            .list()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        for source in installed_sources {
            if available_sources_map
                .get(&source.id)
                .and_then(|index| Version::from_str(&index.version).ok())
                .map(|v| v > source.version)
                .unwrap_or(false)
            {
                for admin in admins.iter() {
                    let message = format!("{} extension update available", source.name);
                    if let Some(chat_id) = admin.telegram_chat_id {
                        if let Err(e) = self
                            .worker_tx
                            .send(WorkerCommand::TelegramMessage(chat_id, message.clone()))
                            .await
                        {
                            error!("failed to send message, reason: {}", e);
                        }
                    }

                    if let Some(user_key) = admin.pushover_user_key.clone() {
                        if let Err(e) = self
                            .worker_tx
                            .send(WorkerCommand::PushoverMessage {
                                user_key,
                                title: None,
                                body: message.clone(),
                            })
                            .await
                        {
                            error!("failed to send PushoverMessage, reason: {}", e);
                        }
                    }
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
            .get("https://api.github.com/repos/faldez/tanoshi/releases/latest")
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
            let admins = self.userdb.get_admins().await?;
            for admin in admins {
                let msg = format!("Tanoshi {} Released\n{}", release.tag_name, release.body);

                if let Some(chat_id) = admin.telegram_chat_id {
                    if let Err(e) = self
                        .worker_tx
                        .send(WorkerCommand::TelegramMessage(chat_id, msg.clone()))
                        .await
                    {
                        error!("failed to send message, reason: {}", e);
                    }
                }

                if let Some(user_key) = admin.pushover_user_key.clone() {
                    if let Err(e) = self
                        .worker_tx
                        .send(WorkerCommand::PushoverMessage {
                            user_key,
                            title: None,
                            body: msg.clone(),
                        })
                        .await
                    {
                        error!("failed to send PushoverMessage, reason: {}", e);
                    }
                }
            }
        } else {
            info!("no tanoshi update found");
        }

        Ok(())
    }

    async fn run(&self) {
        let period = if self.period == 0 { 3600 } else { self.period };
        let mut chapter_update_interval = time::interval(time::Duration::from_secs(period));
        let mut server_update_interval = time::interval(time::Duration::from_secs(86400));

        loop {
            tokio::select! {
                start = chapter_update_interval.tick() => {
                    if self.period == 0 {
                        continue;
                    }

                    info!("start periodic updates");

                    if let Err(e) = self.check_chapter_update().await {
                        error!("failed check chapter update: {}", e)
                    }

                    info!("periodic updates done in {:?}", Instant::now() - start);
                }
                _ = server_update_interval.tick() => {
                    info!("check server update");

                    if let Err(e) = self.check_server_update().await {
                        error!("failed check server update: {}", e)
                    }

                    info!("check extension update");

                    if let Err(e) = self.check_extension_update().await {
                        error!("failed check extension update: {}", e)
                    }
                }
            }
        }
    }
}

pub fn start(
    period: u64,
    userdb: UserDatabase,
    mangadb: MangaDatabase,
    extensions: ExtensionBus,
    worker_tx: Sender<WorkerCommand>,
) -> JoinHandle<()> {
    let worker = UpdatesWorker::new(period, userdb, mangadb, extensions, worker_tx);

    tokio::spawn(async move {
        worker.run().await;
    })
}
