use crate::{
    db::{model::DownloadQueue, MangaDatabase},
    user,
    worker::Command as WorkerCommand,
};

use async_graphql::{Context, Object, Result};
use chrono::Utc;
use tanoshi_vm::prelude::ExtensionBus;
use tokio::sync::mpsc::Sender;

#[derive(Default)]
pub struct DownloadMutationRoot;

#[Object]
impl DownloadMutationRoot {
    async fn download_chapter(&self, ctx: &Context<'_>, id: i64) -> Result<i64> {
        if !user::check_is_admin(ctx)? {
            return Err("Forbidden".into());
        }

        let db = ctx.data::<MangaDatabase>()?;
        let chapter = db.get_chapter_by_id(id).await?;
        let manga = db.get_manga_by_id(chapter.manga_id).await?;
        let pages = db.get_pages_by_chapter_id(id).await?;

        let ext = ctx.data::<ExtensionBus>()?;
        let source = ext.detail(manga.source_id).await?;

        let mut queue = vec![];
        let date_added = Utc::now().naive_utc();
        for (rank, page) in pages.iter().enumerate() {
            queue.push(DownloadQueue {
                id: 0,
                source_name: source.name.clone(),
                manga_title: manga.title.clone(),
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

        Ok(queue.len() as _)
    }
}
