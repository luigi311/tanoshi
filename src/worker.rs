use std::collections::HashMap;

use tanoshi_vm::prelude::ExtensionBus;
use tokio::time::{self, Instant};

use crate::{catalogue::Chapter, db::MangaDatabase};

#[derive(Debug, Clone)]
struct ChapterUpdate {
    manga_title: String,
    cover_url: String,
    title: String,
}

struct Worker {
    period: u64,
    mangadb: MangaDatabase,
    extension_bus: ExtensionBus,
}

impl Worker {
    fn new(period: u64, mangadb: MangaDatabase, extension_bus: ExtensionBus) -> Self {
        #[cfg(not(debug_assertions))]
        let period = if period < 3600 { 3600 } else { period };
        info!("periodic updates every {} secons", period);
        Self {
            period,
            mangadb,
            extension_bus,
        }
    }

    async fn run(&self) {
        let mut interval = time::interval(time::Duration::from_secs(self.period));
        interval.tick().await;

        let mut start;
        loop {
            start = interval.tick().await;
            info!("start periodic updates");

            let manga_in_library = match self.mangadb.get_all_library().await {
                Ok(manga) => manga,
                Err(e) => {
                    error!("error get manga in library, reason: {}", e);
                    continue;
                }
            };

            let mut new_manga_chapter: HashMap<i64, Vec<ChapterUpdate>> = HashMap::new();
            let mut new_users_chapters: HashMap<i64, Vec<ChapterUpdate>> = HashMap::new();

            for (user_id, manga) in manga_in_library {
                if let Some(chapters) = new_manga_chapter.get(&manga.id) {
                    match new_users_chapters.get_mut(&user_id) {
                        Some(user_chapters) => {
                            user_chapters.extend_from_slice(chapters);
                        }
                        None => {
                            new_users_chapters.insert(user_id, chapters.clone());
                        }
                    }
                    continue;
                }

                let last_uploaded_chapter = self
                    .mangadb
                    .get_last_uploaded_chapters_by_manga_id(manga.id)
                    .await;
                let chapters = match self
                    .extension_bus
                    .get_chapters(manga.source_id, manga.path.clone())
                    .await
                {
                    Ok(chapters) => {
                        let chapters: Vec<Chapter> = chapters
                            .into_iter()
                            .map(|ch| {
                                let mut c: Chapter = ch.into();
                                c.manga_id = manga.id;
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
                        .filter(|ch| ch.uploaded > last_uploaded_chapter.uploaded)
                        .collect()
                } else {
                    chapters
                };

                let chapters: Vec<ChapterUpdate> = chapters
                    .iter()
                    .map(|ch| ChapterUpdate {
                        manga_title: manga.title.clone(),
                        cover_url: manga.cover_url.clone(),
                        title: ch.title.clone(),
                    })
                    .collect();

                new_manga_chapter.insert(manga.id, chapters.clone());
                match new_users_chapters.get_mut(&user_id) {
                    Some(user_chapters) => {
                        user_chapters.extend_from_slice(&chapters);
                    }
                    None => {
                        new_users_chapters.insert(user_id, chapters);
                    }
                }
            }

            info!("users' new chapters: {:?}", new_users_chapters);
            info!("periodic updates done in {:?}", Instant::now() - start);
        }
    }
}

pub fn start(
    period: u64,
    mangadb: MangaDatabase,
    extension_bus: ExtensionBus,
) {
    let worker = Worker::new(period, mangadb, extension_bus);

    tokio::spawn(async move {
        worker.run().await;
    });
}
