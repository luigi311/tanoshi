use crate::user::User;
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

    pub async fn insert_user(&self, user: User) -> Result<i64> {
        let role: u8 = user.role.into();
        let row_id = sqlx::query(
            r#"INSERT INTO user(
                username,
                password,
                role
            ) VALUES (?, ?, ?)"#,
        )
        .bind(&user.username)
        .bind(&user.password)
        .bind(role)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        Ok(row_id)
    }

    pub async fn update_password(&self, id: i64, password: String) -> Result<u64> {
        let row_id = sqlx::query(
            r#"UPDATE user
                SET password = ?
                WHERE id = ?"#,
        )
        .bind(&password)
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(row_id)
    }

    pub async fn get_users(&self) -> Result<Vec<User>> {
        let mut stream = sqlx::query(r#"SELECT * FROM user"#).fetch(&self.pool);

        let mut users = vec![];
        while let Some(row) = stream.try_next().await? {
            users.push(User {
                id: row.get(0),
                username: row.get(1),
                password: row.get(2),
                role: row.get::<u8, _>(3).into(),
            })
        }

        Ok(users)
    }

    pub async fn get_users_count(&self) -> Result<i64> {
        let stream = sqlx::query(r#"SELECT COUNT(1) FROM user"#)
            .fetch_one(&self.pool)
            .await
            .ok();

        if let Some(row) = stream {
            Ok(row.get(0))
        } else {
            Err(anyhow!("Not found"))
        }
    }

    pub async fn get_user_by_id(&self, id: i64) -> Result<User> {
        let stream = sqlx::query(r#"SELECT * FROM user WHERE id = ?"#)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .ok();

        if let Some(row) = stream {
            Ok(User {
                id: row.get(0),
                username: row.get(1),
                password: row.get(2),
                role: row.get::<u8, _>(3).into(),
            })
        } else {
            Err(anyhow!("Not found"))
        }
    }

    pub async fn get_user_by_username(&self, username: String) -> Result<User> {
        let stream = sqlx::query(r#"SELECT * FROM user WHERE username = ?"#)
            .bind(&username)
            .fetch_one(&self.pool)
            .await
            .ok();

        if let Some(row) = stream {
            Ok(User {
                id: row.get(0),
                username: row.get(1),
                password: row.get(2),
                role: row.get::<u8, _>(3).into(),
            })
        } else {
            Err(anyhow!("Not found"))
        }
    }
}
