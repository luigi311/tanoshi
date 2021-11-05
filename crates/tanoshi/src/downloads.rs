use std::path::PathBuf;

use crate::{
    db::{model, MangaDatabase},
    local, user,
    worker::Command as WorkerCommand,
};

use async_graphql::{Context, Object, Result, SimpleObject};
use chrono::Utc;
use tanoshi_vm::prelude::ExtensionBus;
use tokio::sync::mpsc::Sender;

#[derive(Debug, SimpleObject)]
pub struct DownloadQueueStatus {
    source_name: String,
    manga_title: String,
    chapter_title: String,
    downloaded: i64,
    total: i64,
}

#[derive(Default)]
pub struct DownloadRoot;

#[Object]
impl DownloadRoot {
    async fn download_queue(&self, ctx: &Context<'_>) -> Result<Vec<DownloadQueueStatus>> {
        if !user::check_is_admin(ctx)? {
            return Err("Forbidden".into());
        }

        let db = ctx.data::<MangaDatabase>()?;
        let queue: Vec<model::DownloadQueueStatus> = db.get_download_queue().await?;
        let queue = queue
            .iter()
            .map(|queue| DownloadQueueStatus {
                source_name: queue.source_name.clone(),
                manga_title: queue.manga_title.clone(),
                chapter_title: queue.chapter_title.clone(),
                downloaded: queue.downloaded,
                total: queue.total,
            })
            .collect();

        Ok(queue)
    }
}
#[derive(Default)]
pub struct DownloadMutationRoot;

#[Object]
impl DownloadMutationRoot {
    async fn download_chapters(&self, ctx: &Context<'_>, ids: Vec<i64>) -> Result<i64> {
        if !user::check_is_admin(ctx)? {
            return Err("Forbidden".into());
        }

        let db = ctx.data::<MangaDatabase>()?;
        let ext = ctx.data::<ExtensionBus>()?;

        let mut len = 0_usize;
        for id in ids {
            let chapter = db.get_chapter_by_id(id).await?;
            if chapter.source_id == local::ID {
                info!("local source can't be downloaded");
                continue;
            }

            let manga = db.get_manga_by_id(chapter.manga_id).await?;
            let pages = match db.get_pages_remote_url_by_chapter_id(id).await {
                Ok(pages) => pages,
                Err(_) => {
                    let pages = ext.get_pages(manga.source_id, chapter.path.clone()).await?;
                    db.insert_pages(chapter.id, &pages).await?;
                    pages
                }
            };

            let source = ext.detail(manga.source_id).await?;

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
            db.insert_download_queue(&queue).await?;
            ctx.data::<Sender<WorkerCommand>>()?
                .send(WorkerCommand::StartDownload)
                .await?;

            len += queue.len()
        }

        Ok(len as _)
    }

    async fn remove_downloaded_chapters(&self, ctx: &Context<'_>, ids: Vec<i64>) -> Result<i64> {
        if !user::check_is_admin(ctx)? {
            return Err("Forbidden".into());
        }

        let db = ctx.data::<MangaDatabase>()?;

        let mut len = 0_usize;
        for id in ids {
            let pages = db.get_pages_local_url_by_chapter_id(id).await?;
            for page in pages {
                if page.is_empty() {
                    continue;
                }
                let page = PathBuf::new().join(page);

                info!("removing {}...", page.display());
                tokio::fs::remove_file(&page).await?;
                db.delete_page_local_url(&page.display().to_string())
                    .await?;
                len += 1;
            }
        }

        Ok(len as _)
    }
}
