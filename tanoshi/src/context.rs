use crate::catalogue::*;
use crate::extension::Extensions;
use crate::db::Db;

pub struct GlobalContext {
    pub db: Db,
    pub extensions: Extensions,
}

impl GlobalContext {
    pub fn new(db: Db, extensions: Extensions) -> Self {
        Self { db, extensions }
    }
}
