use crate::catalogue::{CatalogueRoot, SourceRoot};
use crate::library::{LibraryRoot, LibraryMutationRoot};
use crate::user::{UserRoot, UserMutationRoot};
use async_graphql::{
    EmptySubscription, MergedObject,Schema
};

pub type TanoshiSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(MergedObject, Default)]
pub struct QueryRoot(SourceRoot, CatalogueRoot, LibraryRoot, UserRoot);

#[derive(MergedObject, Default)]
pub struct MutationRoot(LibraryMutationRoot, UserMutationRoot);
