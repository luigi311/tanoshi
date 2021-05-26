use crate::db::{MangaDatabase, UserDatabase};
use crate::extension::Extensions;

pub struct GlobalContext {
    pub userdb: UserDatabase,
    pub mangadb: MangaDatabase,
    pub extensions: Extensions,
}

impl GlobalContext {
    pub fn new(userdb: UserDatabase, mangadb: MangaDatabase, extensions: Extensions) -> Self {
        Self {
            userdb,
            mangadb,
            extensions,
        }
    }
}
