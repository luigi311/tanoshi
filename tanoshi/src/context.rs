use crate::catalogue::*;
use crate::extension::Extensions;
use crate::db::MangaDatabase;

pub struct GlobalContext {
    pub mangadb: MangaDatabase,
    pub extensions: Extensions,
}

impl GlobalContext {
    pub fn new(mangadb: MangaDatabase, extensions: Extensions) -> Self {
        Self { mangadb, extensions }
    }
}
