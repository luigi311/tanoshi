use crate::catalogue::{CatalogueRoot, SourceRoot};
use crate::library::{LibraryRoot, LibraryMutationRoot};
use async_graphql::{
    Context, EmptyMutation, EmptySubscription, MergedObject, Object, Result, Schema, Subscription,
    ID,
};

pub type TanoshiSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(MergedObject, Default)]
pub struct QueryRoot(SourceRoot, CatalogueRoot, LibraryRoot);

#[derive(MergedObject, Default)]
pub struct MutationRoot(LibraryMutationRoot);
