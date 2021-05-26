use crate::catalogue::{Chapter, Manga, Page};
use crate::library::{RecentChapter, RecentUpdate};
use anyhow::{anyhow, Result};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub fn new(pool: SqlitePool) -> Db {
        Db { pool }
    }
}