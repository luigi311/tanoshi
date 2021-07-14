use sqlx::{
    migrate::MigrateDatabase,
    sqlite::{SqlitePool, SqlitePoolOptions},
    Sqlite,
};

mod manga;
pub use manga::Db as MangaDatabase;

mod user;
pub use user::Db as UserDatabase;

pub async fn establish_connection(database_path: String) -> SqlitePool {
    if !Sqlite::database_exists(&database_path).await.unwrap() {
        Sqlite::create_database(&database_path).await.unwrap();
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(25)
        .connect(&database_path)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}
