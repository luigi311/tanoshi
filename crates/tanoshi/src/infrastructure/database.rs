use std::ops::{Deref, DerefMut};

use sqlx::{
    migrate::MigrateError,
    sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteJournalMode},
};

#[derive(Clone)]
pub struct Pool(SqlitePool);

impl From<SqlitePool> for Pool {
    fn from(pool: SqlitePool) -> Self {
        Self(pool)
    }
}

impl Deref for Pool {
    type Target = SqlitePool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Pool {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub async fn establish_connection(
    database_path: &str,
    create: bool,
) -> Result<Pool, anyhow::Error> {
    let opts = SqliteConnectOptions::new()
        .create_if_missing(create)
        .filename(database_path)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .idle_timeout(std::time::Duration::from_secs(60))
        .max_lifetime(std::time::Duration::from_secs(3 * 60))
        .connect_with(opts)
        .await?;

    match sqlx::migrate!("./migrations").run(&pool).await {
        Err(MigrateError::VersionMismatch(version)) => {
            warn!("migration {version} was previously applied but has been modified")
        }
        Err(e) => {
            return Err(e.into());
        }
        _ => {}
    }

    Ok(Pool(pool))
}
