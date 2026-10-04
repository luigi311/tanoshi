use std::collections::HashSet;

use async_trait::async_trait;
use chrono::Utc;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use sqlx::Row;

use crate::{
    domain::{
        entities::chapter::Chapter,
        repositories::chapter::{ChapterRepository, ChapterRepositoryError},
    },
    infrastructure::database::Pool,
};

// Leave headroom below SQLite's variable limit, including eight bindings per insert.
const CHAPTER_CHUNK_SIZE: usize = 1_000;

#[derive(Clone)]
pub struct ChapterRepositoryImpl {
    pool: Pool,
}

impl ChapterRepositoryImpl {
    pub fn new<P: Into<Pool>>(pool: P) -> Self {
        Self { pool: pool.into() }
    }
}

#[async_trait]
impl ChapterRepository for ChapterRepositoryImpl {
    async fn insert_chapters(&self, chapters: &[Chapter]) -> Result<(), ChapterRepositoryError> {
        if chapters.is_empty() {
            return Ok(());
        }

        let date_added = Utc::now().naive_utc();

        // Keep the whole refresh atomic even if a later chunk fails.
        let mut tx = self.pool.write().begin().await?;
        for chunk in chapters.chunks(CHAPTER_CHUNK_SIZE) {
            let values = vec!["(?, ?, ?, ?, ?, ?, ?, ?)"; chunk.len()];

            let query_str = format!(
                r#"INSERT INTO chapter(
                    source_id,
                    manga_id,
                    title,
                    path,
                    number,
                    scanlator,
                    uploaded,
                    date_added
                ) VALUES {} ON CONFLICT(source_id, path) DO UPDATE SET
                    manga_id=excluded.manga_id,
                    title=excluded.title,
                    number=excluded.number,
                    scanlator=excluded.scanlator,
                    uploaded=excluded.uploaded
                WHERE (chapter.title, chapter.number, chapter.scanlator, chapter.uploaded, chapter.manga_id)
                    IS NOT (excluded.title, excluded.number, excluded.scanlator, excluded.uploaded, excluded.manga_id)
                "#,
                values.join(",")
            );

            let mut query = sqlx::query(&query_str);
            for chapter in chunk {
                query = query
                    .bind(chapter.source_id)
                    .bind(chapter.manga_id)
                    .bind(&chapter.title)
                    .bind(&chapter.path)
                    .bind(chapter.number)
                    .bind(&chapter.scanlator)
                    .bind(chapter.uploaded)
                    .bind(date_added);
            }

            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    async fn get_chapter_by_id(&self, id: i64) -> Result<Chapter, ChapterRepositoryError> {
        let row = sqlx::query(
            r#"SELECT 
                        chapter.*,
                        (SELECT c.id FROM chapter c WHERE c.manga_id = chapter.manga_id AND c.number > chapter.number ORDER BY c.number ASC LIMIT 1) next,
                        (SELECT c.id FROM chapter c WHERE c.manga_id = chapter.manga_id AND c.number < chapter.number ORDER BY c.number DESC LIMIT 1) prev
                    FROM chapter WHERE id = ?"#,
        )
        .bind(id)
        .fetch_one(self.pool.read())
        .await?;

        Ok(Chapter {
            id: row.get(0),
            source_id: row.get(1),
            manga_id: row.get(2),
            title: row.get(3),
            path: row.get(4),
            number: row.get(5),
            scanlator: row.get(6),
            uploaded: row.get(7),
            date_added: row.get(8),
            downloaded_path: row.get(9),
            next: row.get(10),
            prev: row.get(11),
        })
    }

    async fn get_chapter_by_source_id_path(
        &self,
        source_id: i64,
        path: &str,
    ) -> Result<Chapter, ChapterRepositoryError> {
        let row = sqlx::query(
            r#"SELECT 
                        chapter.*,
                        (SELECT c.id FROM chapter c WHERE c.manga_id = chapter.manga_id AND c.number > chapter.number ORDER BY c.number ASC LIMIT 1) next,
                        (SELECT c.id FROM chapter c WHERE c.manga_id = chapter.manga_id AND c.number < chapter.number ORDER BY c.number DESC LIMIT 1) prev
                    FROM chapter WHERE source_id = ? AND path = ?"#,
        )
        .bind(source_id)
        .bind(path)
        .fetch_one(self.pool.read())
        .await?;

        Ok(Chapter {
            id: row.get(0),
            source_id: row.get(1),
            manga_id: row.get(2),
            title: row.get(3),
            path: row.get(4),
            number: row.get(5),
            scanlator: row.get(6),
            uploaded: row.get(7),
            date_added: row.get(8),
            downloaded_path: row.get(9),
            next: row.get(10),
            prev: row.get(11),
        })
    }

    async fn get_chapters_by_manga_id(
        &self,
        manga_id: i64,
        limit: Option<i64>,
        order_by: Option<&'static str>,
        asc: bool,
    ) -> Result<Vec<Chapter>, ChapterRepositoryError> {
        let limit = limit
            .map(|limit| format!("LIMIT {limit}"))
            .unwrap_or_default();
        let order_by = order_by.unwrap_or("number");
        let order = if asc { "ASC" } else { "DESC" };

        let query_str = format!(
            r#"SELECT
                        chapter.*,
                        (SELECT c.id FROM chapter c WHERE c.manga_id = chapter.manga_id AND c.number > chapter.number ORDER BY c.number ASC LIMIT 1) next,
                        (SELECT c.id FROM chapter c WHERE c.manga_id = chapter.manga_id AND c.number < chapter.number ORDER BY c.number DESC LIMIT 1) prev
                    FROM chapter WHERE manga_id = ? ORDER BY {order_by} {order} {limit}"#,
        );
        let chapters = sqlx::query(&query_str)
            .bind(manga_id)
            .fetch_all(self.pool.read())
            .await?
            .into_par_iter()
            .map(|row| Chapter {
                id: row.get(0),
                source_id: row.get(1),
                manga_id: row.get(2),
                title: row.get(3),
                path: row.get(4),
                number: row.get(5),
                scanlator: row.get(6),
                uploaded: row.get(7),
                date_added: row.get(8),
                downloaded_path: row.get(9),
                next: row.get(10),
                prev: row.get(11),
            })
            .collect();

        Ok(chapters)
    }

    async fn delete_chapter_by_id(&self, chapter_id: i64) -> Result<(), ChapterRepositoryError> {
        sqlx::query("DELETE FROM chapter WHERE id = ?")
            .bind(chapter_id)
            .execute(self.pool.write())
            .await?;

        Ok(())
    }

    async fn delete_chapter_by_ids(
        &self,
        chapter_ids: &[i64],
    ) -> Result<(), ChapterRepositoryError> {
        if chapter_ids.is_empty() {
            return Err(ChapterRepositoryError::BadArgsError(
                "chapter_ids should at least be 1".to_string(),
            ));
        }

        let mut tx = self.pool.write().begin().await?;
        for chunk in chapter_ids.chunks(CHAPTER_CHUNK_SIZE) {
            let query_str = format!(
                "DELETE FROM chapter WHERE id IN ({})",
                vec!["?"; chunk.len()].join(",")
            );

            let mut query = sqlx::query(&query_str);

            for chapter_id in chunk {
                query = query.bind(chapter_id);
            }

            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    async fn get_chapters_not_in_source(
        &self,
        source_id: i64,
        manga_id: i64,
        paths: &[String],
    ) -> Result<Vec<Chapter>, ChapterRepositoryError> {
        if paths.is_empty() {
            return Err(ChapterRepositoryError::BadArgsError(
                "paths should at least be 1".to_string(),
            ));
        }

        // Compare against the complete path set: separate NOT IN queries would
        // include chapters whose paths occur in a different chunk.
        let paths: HashSet<&str> = paths.iter().map(String::as_str).collect();
        // Both reads use one snapshot so chapters cannot change between them.
        let mut tx = self.pool.read().begin().await?;
        let chapter_ids: Vec<i64> =
            sqlx::query("SELECT id, path FROM chapter WHERE source_id = ? AND manga_id = ?")
                .bind(source_id)
                .bind(manga_id)
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .filter(|row| !paths.contains(row.get::<&str, _>("path")))
                .map(|row| row.get("id"))
                .collect();

        let mut chapters = Vec::new();
        for chunk in chapter_ids.chunks(CHAPTER_CHUNK_SIZE) {
            let query_str = format!(
                "SELECT * FROM chapter WHERE id IN ({})",
                vec!["?"; chunk.len()].join(",")
            );
            let mut query = sqlx::query(&query_str);
            for chapter_id in chunk {
                query = query.bind(chapter_id);
            }
            chapters.extend(
                query
                    .fetch_all(&mut *tx)
                    .await?
                    .into_par_iter()
                    .map(|row| Chapter {
                        id: row.get(0),
                        source_id: row.get(1),
                        manga_id: row.get(2),
                        title: row.get(3),
                        path: row.get(4),
                        number: row.get(5),
                        scanlator: row.get(6),
                        uploaded: row.get(7),
                        date_added: row.get(8),
                        downloaded_path: row.get(9),
                        next: None,
                        prev: None,
                    })
                    .collect::<Vec<_>>(),
            );
        }
        tx.commit().await?;

        Ok(chapters)
    }
}

#[cfg(test)]
mod tests;
