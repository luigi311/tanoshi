use sqlx::{
    migrate::MigrateError,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
};

const SQLITE_READ_CONNECTIONS: u32 = 4;
const SQLITE_WRITE_CONNECTIONS: u32 = 1;

/// Shared database access with separate capacity for readers and writers.
#[derive(Clone)]
pub struct Pool {
    read: SqlitePool,
    write: SqlitePool,
}

impl Pool {
    /// Ordinary reads use connections that cannot modify the database.
    pub fn read(&self) -> &SqlitePool {
        &self.read
    }

    /// Writes and all queries belonging to a write transaction use this pool.
    pub fn write(&self) -> &SqlitePool {
        &self.write
    }

    pub async fn close(&self) {
        tokio::join!(self.read.close(), self.write.close());
    }
}

fn sqlite_pool_options(max_connections: u32) -> SqlitePoolOptions {
    SqlitePoolOptions::new()
        .max_connections(max_connections)
        .idle_timeout(std::time::Duration::from_secs(60))
        .max_lifetime(std::time::Duration::from_secs(3 * 60))
}

pub async fn establish_connection(
    database_path: &str,
    create: bool,
) -> Result<Pool, anyhow::Error> {
    let opts = SqliteConnectOptions::new()
        .create_if_missing(create)
        .filename(database_path)
        .journal_mode(SqliteJournalMode::Wal);

    // SQLite admits only one writer. Waiting writers never consume reader slots.
    let write = sqlite_pool_options(SQLITE_WRITE_CONNECTIONS)
        .connect_with(opts)
        .await?;

    match sqlx::migrate!("./migrations").run(&write).await {
        Err(MigrateError::VersionMismatch(version)) => {
            warn!("migration {version} was previously applied but has been modified");
        }
        Err(e) => {
            error!("database migration failed: {e}");
            write.close().await;
            return Err(e.into());
        }
        _ => {
            info!("database migrations applied successfully");
        }
    }

    // Open readers after migrations. They inherit the database's WAL mode;
    // setting journal_mode on a read-only connection could require a write.
    let read_opts = SqliteConnectOptions::new()
        .filename(database_path)
        .read_only(true)
        .pragma("query_only", "ON");
    let read = match sqlite_pool_options(SQLITE_READ_CONNECTIONS)
        .connect_with(read_opts)
        .await
    {
        Ok(read) => read,
        Err(e) => {
            write.close().await;
            return Err(e.into());
        }
    };

    Ok(Pool { read, write })
}

#[cfg(test)]
mod tests;
