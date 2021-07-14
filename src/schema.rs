use crate::catalogue::{CatalogueRoot, SourceRoot, SourceMutationRoot};
use crate::library::{LibraryRoot, LibraryMutationRoot};
use crate::status::StatusRoot;
use crate::user::{UserRoot, UserMutationRoot};
use async_graphql::{
    EmptySubscription, MergedObject,Schema
};

pub type TanoshiSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(MergedObject, Default)]
pub struct QueryRoot(SourceRoot, CatalogueRoot, LibraryRoot, UserRoot, StatusRoot);

#[derive(MergedObject, Default)]
pub struct MutationRoot(LibraryMutationRoot, UserMutationRoot, SourceMutationRoot);
