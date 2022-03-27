use crate::{
    catalogue::{
        chapter::{MangaLoader, NextChapterLoader, PrevChapterLoader, ReadProgressLoader},
        manga::{FavoriteLoader, UserLastReadLoader, UserUnreadChaptersLoader},
        CatalogueRoot, SourceMutationRoot, SourceRoot,
    },
    db::{MangaDatabase, UserDatabase},
    downloads::{DownloadMutationRoot, DownloadRoot},
    library::{CategoryMutationRoot, CategoryRoot, LibraryMutationRoot, LibraryRoot},
    notification::NotificationRoot,
    notifier::Notifier,
    status::StatusRoot,
    tracker::MyAnimeList,
    tracking::{TrackingMutationRoot, TrackingRoot},
    user::{UserMutationRoot, UserRoot},
    worker::downloads::DownloadSender,
};
use tanoshi_vm::extension::SourceBus;

use async_graphql::{dataloader::DataLoader, EmptySubscription, MergedObject, Schema};

pub type TanoshiSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(MergedObject, Default)]
pub struct QueryRoot(
    SourceRoot,
    CatalogueRoot,
    LibraryRoot,
    CategoryRoot,
    UserRoot,
    StatusRoot,
    NotificationRoot,
    DownloadRoot,
    TrackingRoot,
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    LibraryMutationRoot,
    CategoryMutationRoot,
    UserMutationRoot,
    SourceMutationRoot,
    DownloadMutationRoot,
    TrackingMutationRoot,
);

pub fn build(
    userdb: UserDatabase,
    mangadb: MangaDatabase,
    ext_manager: SourceBus,
    download_tx: DownloadSender,
    notifier: Notifier,
    mal_client: Option<MyAnimeList>,
) -> TanoshiSchema {
    let mut builder = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription::default(),
    )
    // .extension(ApolloTracing)
    .data(DataLoader::new(
        FavoriteLoader {
            mangadb: mangadb.clone(),
        },
        tokio::spawn,
    ))
    .data(DataLoader::new(
        UserLastReadLoader {
            mangadb: mangadb.clone(),
        },
        tokio::spawn,
    ))
    .data(DataLoader::new(
        UserUnreadChaptersLoader {
            mangadb: mangadb.clone(),
        },
        tokio::spawn,
    ))
    .data(DataLoader::new(
        ReadProgressLoader {
            mangadb: mangadb.clone(),
        },
        tokio::spawn,
    ))
    .data(DataLoader::new(
        PrevChapterLoader {
            mangadb: mangadb.clone(),
        },
        tokio::spawn,
    ))
    .data(DataLoader::new(
        NextChapterLoader {
            mangadb: mangadb.clone(),
        },
        tokio::spawn,
    ))
    .data(DataLoader::new(
        MangaLoader {
            mangadb: mangadb.clone(),
        },
        tokio::spawn,
    ))
    .data(userdb)
    .data(mangadb)
    .data(ext_manager)
    .data(notifier)
    .data(download_tx);

    if let Some(mal_client) = mal_client {
        builder = builder.data(mal_client);
    }

    builder.finish()
}
