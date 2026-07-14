//! Shared composition root for the server binary and the desktop app.
//!
//! Wires repositories, services and background workers from a [`Config`] and
//! returns a ready-to-serve [`ServerBuilder`]; the caller picks the listen
//! address and drives the returned worker handles.

use anyhow::Result;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::time::Duration;
use tokio::task::JoinHandle;

use tanoshi_notifier::{gotify::Gotify, pushover::Pushover, telegram::Telegram};
use tanoshi_tracker::{AniList, MyAnimeList};
use tanoshi_vm::{
    extension::{ExtensionManager, ExtensionManagerOptions},
    prelude::Source,
};

use crate::{
    application::worker,
    domain::services::{
        chapter::ChapterService, download::DownloadService, history::HistoryService,
        image::ImageService, library::LibraryService, manga::MangaService, source::SourceService,
        tracker::TrackerService, user::UserService,
    },
    infrastructure::{
        config::{self, Config},
        database,
        domain::repositories::{
            chapter::ChapterRepositoryImpl, download::DownloadRepositoryImpl,
            history::HistoryRepositoryImpl, image::ImageRepositoryImpl,
            image_cache::ImageCacheRepositoryImpl, library::LibraryRepositoryImpl,
            manga::MangaRepositoryImpl, source::SourceRepositoryImpl,
            tracker::TrackerRepositoryImpl, user::UserRepositoryImpl,
        },
        local, notification,
    },
    presentation::{graphql::loader::DatabaseLoader, ServerBuilder},
};

/// A fully wired application: background workers running and every service
/// registered on the [`ServerBuilder`].
pub struct App {
    pub pool: database::Pool,
    pub server_builder: ServerBuilder,
    pub update_worker_handle: JoinHandle<()>,
    pub download_worker_handle: JoinHandle<()>,
    /// Long-running Telegram bot loop; present when telegram is configured.
    /// Callers must poll or spawn it for Telegram notifications to work.
    pub telegram_bot: Option<BoxFuture<'static, ()>>,
}

pub async fn bootstrap(config: Config) -> Result<App> {
    info!("connecting to database at {}", config.database_path);
    let pool =
        database::establish_connection(&config.database_path, config.create_database).await?;
    info!("database connected");

    let user_repo = UserRepositoryImpl::new(pool.clone());
    let user_svc = UserService::new(user_repo.clone());

    let extension_manager = ExtensionManager::new_with_options(
        &config.plugin_path,
        ExtensionManagerOptions {
            max_concurrent_calls: config.extension.max_concurrent_calls_per_source,
            admission_timeout: Duration::from_millis(config.extension.admission_timeout_ms),
            metadata_timeout: Duration::from_secs(config.extension.metadata_timeout_secs),
            image_timeout: Duration::from_secs(config.extension.image_timeout_secs),
        },
    );

    info!("loading extensions from {}", config.plugin_path);
    extension_manager.load_all().await?;

    let source_repo = SourceRepositoryImpl::new(extension_manager.clone());
    let source_svc = SourceService::new(source_repo);

    let manga_repo = MangaRepositoryImpl::new(pool.clone());
    let manga_svc = MangaService::new(manga_repo.clone(), extension_manager.clone());

    let chapter_repo = ChapterRepositoryImpl::new(pool.clone());
    let chapter_svc = ChapterService::new(chapter_repo.clone(), extension_manager.clone());

    let library_repo = LibraryRepositoryImpl::new(pool.clone());
    let library_svc = LibraryService::new(library_repo.clone());

    let history_repo = HistoryRepositoryImpl::new(pool.clone());
    let history_svc = HistoryService::new(chapter_repo.clone(), history_repo.clone());

    match &config.local_path {
        config::LocalFolders::Single(local_path) => {
            extension_manager
                .insert(Source::from(Box::new(local::Local::new(
                    10000,
                    "Local".to_string(),
                    local_path,
                ))))
                .await?;
        }
        config::LocalFolders::Multiple(local_paths) => {
            for (index, local_path) in local_paths.iter().enumerate() {
                // source id starts from 10000
                let index = index + 10000;
                extension_manager
                    .insert(Source::from(Box::new(local::Local::new(
                        index as i64,
                        local_path.name.clone(),
                        &local_path.path,
                    ))))
                    .await?;
            }
        }
    }

    let mut notifier_builder = notification::Builder::new(user_repo.clone());

    let mut telegram_bot: Option<BoxFuture<'static, ()>> = None;
    if let Some(telegram_config) = config.telegram.clone() {
        let bot = Telegram::new(telegram_config.token);
        telegram_bot = Some(tanoshi_notifier::telegram::run(bot.clone()).boxed());
        notifier_builder = notifier_builder.telegram(bot);
    }

    if let Some(pushover_cfg) = config.pushover.as_ref() {
        notifier_builder =
            notifier_builder.pushover(Pushover::new(pushover_cfg.application_key.clone()));
    }

    if let Some(gotify_cfg) = config.gotify.as_ref() {
        notifier_builder = notifier_builder.gotify(Gotify::new(gotify_cfg.base_url.clone()));
    }

    if let Some(base_url) = config.base_url.as_ref() {
        notifier_builder = notifier_builder.base_url(base_url.clone());
    }

    let notifier = notifier_builder.finish();

    // validate tracker configuration before spawning workers so a config
    // error cannot leave detached workers running against the pool
    let mal_client = if let Some(mal_cfg) = config.myanimelist.as_ref() {
        if let Some(base_url) = config.base_url.as_ref() {
            MyAnimeList::new(
                base_url,
                mal_cfg.client_id.clone(),
                mal_cfg.client_secret.clone(),
            )
            .ok()
        } else {
            return Err(anyhow::anyhow!(
                "Invalid config: MyAnimeList tracker needs base_url to login"
            ));
        }
    } else {
        None
    };

    let al_client = if let Some(al_cfg) = config.anilist.as_ref() {
        if let Some(base_url) = config.base_url.as_ref() {
            AniList::new(
                base_url,
                al_cfg.client_id.clone(),
                al_cfg.client_secret.clone(),
            )
            .ok()
        } else {
            return Err(anyhow::anyhow!(
                "Invalid config: AniList tracker needs base_url to login"
            ));
        }
    } else {
        None
    };

    let (chapter_update_receiver, chapter_update_command_tx, update_worker_handle) =
        worker::updates::start(
            config.update_interval,
            config.max_concurrent_update_sources,
            library_repo.clone(),
            manga_repo.clone(),
            chapter_repo.clone(),
            extension_manager.clone(),
            notifier.clone(),
            config.extension_repository.clone(),
            &config.cache_path,
        );

    let (download_sender, download_receiver) = worker::downloads::channel();

    let download_repo = DownloadRepositoryImpl::new(pool.clone());
    let download_svc = DownloadService::new(download_repo.clone(), download_sender.clone());

    let download_worker_handle = worker::downloads::start(
        &config.download_path,
        chapter_repo.clone(),
        manga_repo.clone(),
        download_repo.clone(),
        library_repo.clone(),
        extension_manager.clone(),
        notifier.clone(),
        download_sender.clone(),
        download_receiver,
        chapter_update_receiver.resubscribe(),
        config.auto_download_chapters,
    );

    let tracker_repo = TrackerRepositoryImpl::new(pool.clone(), mal_client.clone(), al_client);
    let tracker_svc = TrackerService::new(tracker_repo.clone());

    let image_repo = ImageRepositoryImpl::new(extension_manager.clone());
    let image_cache_repo = ImageCacheRepositoryImpl::new(&config.cache_path);
    let image_svc = ImageService::new(image_repo, image_cache_repo);

    let loader = DatabaseLoader::new(
        history_repo,
        library_repo,
        manga_repo,
        tracker_repo,
        download_repo,
    );

    let mut server_builder = ServerBuilder::new()
        .with_config(config.clone())
        .with_user_svc(user_svc)
        .with_tracker_svc(tracker_svc)
        .with_source_svc(source_svc)
        .with_manga_svc(manga_svc)
        .with_chapter_svc(chapter_svc)
        .with_image_svc(image_svc)
        .with_library_svc(library_svc)
        .with_history_svc(history_svc)
        .with_download_svc(download_svc)
        .with_ext_manager(extension_manager)
        .with_download_tx(download_sender)
        .with_notifier(notifier)
        .with_chapter_update_receiver(chapter_update_receiver)
        .with_chapter_update_command_tx(chapter_update_command_tx)
        .with_loader(loader);

    if config.enable_playground {
        server_builder = server_builder.enable_playground();
    }

    info!("bootstrap complete");

    Ok(App {
        pool,
        server_builder,
        update_worker_handle,
        download_worker_handle,
        telegram_bot,
    })
}
