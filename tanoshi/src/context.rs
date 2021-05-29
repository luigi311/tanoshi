use std::sync::{Arc, RwLock};

use crate::db::{MangaDatabase, UserDatabase};
use crate::extension::Extensions;

pub struct GlobalContext {
    pub userdb: UserDatabase,
    pub secret: String,
    pub mangadb: MangaDatabase,
    pub extensions: Arc<RwLock<Extensions>>,
}

impl GlobalContext {
    pub fn new(userdb: UserDatabase, mangadb: MangaDatabase, secret: String, extensions: Extensions) -> Self {
        Self {
            userdb,
            mangadb,
            secret,
            extensions: Arc::new(RwLock::new(extensions)),
        }
    }
}
