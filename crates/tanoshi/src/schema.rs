use crate::{
    catalogue::{
        chapter::{MangaLoader, NextChapterLoader, PrevChapterLoader, ReadProgressLoader},
        manga::{FavoriteLoader, UserLastReadLoader, UserUnreadChaptersLoader},
        CatalogueRoot, SourceMutationRoot, SourceRoot,
    },
    db::{MangaDatabase, UserDatabase},
    downloads::{DownloadMutationRoot, DownloadRoot},
    library::{LibraryMutationRoot, LibraryRoot},
    notification::NotificationRoot,
    notifier::Notifier,
    status::StatusRoot,
    user::{UserMutationRoot, UserRoot},
    worker::downloads::DownloadSender,
};
use tanoshi_vm::bus::ExtensionBus;

use async_graphql::{dataloader::DataLoader, EmptySubscription, MergedObject, Schema};

pub type TanoshiSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(MergedObject, Default)]
pub struct QueryRoot(
    SourceRoot,
    CatalogueRoot,
    LibraryRoot,
    UserRoot,
    StatusRoot,
    NotificationRoot,
    DownloadRoot,
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    LibraryMutationRoot,
    UserMutationRoot,
    SourceMutationRoot,
    DownloadMutationRoot,
);

pub fn build(
    userdb: UserDatabase,
    mangadb: MangaDatabase,
    extension_bus: ExtensionBus,
    download_tx: DownloadSender,
    notifier: Notifier,
) -> TanoshiSchema {
    let schemabuilder = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription::default(),
    )
    // .extension(ApolloTracing)
    .data(DataLoader::new(FavoriteLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(UserLastReadLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(UserUnreadChaptersLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(ReadProgressLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(PrevChapterLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(NextChapterLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(MangaLoader {
        mangadb: mangadb.clone(),
    }))
    .data(userdb)
    .data(mangadb)
    .data(extension_bus)
    .data(notifier)
    .data(download_tx);

    schemabuilder.finish()
}
