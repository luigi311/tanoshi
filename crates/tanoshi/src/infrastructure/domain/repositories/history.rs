use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use sqlx::{QueryBuilder, Row, Sqlite};

use crate::{
    domain::{
        entities::history::{HistoryBounds, HistoryChapter, HistoryCursor, HistoryPageInfo},
        repositories::history::{HistoryRepository, HistoryRepositoryError},
    },
    infrastructure::database::Pool,
};

const HISTORY_PAGE_INFO_SQL: &str = r#"
    SELECT
        EXISTS(SELECT 1 FROM user_manga_history
            WHERE user_id = ? AND (read_at, manga_id) > (?, ?)),
        EXISTS(SELECT 1 FROM user_manga_history
            WHERE user_id = ? AND (read_at, manga_id) < (?, ?))
"#;

#[derive(Clone)]
pub struct HistoryRepositoryImpl {
    pool: Pool,
}

impl HistoryRepositoryImpl {
    pub fn new<P: Into<Pool>>(pool: P) -> Self {
        Self { pool: pool.into() }
    }

    async fn history_chapters(
        &self,
        user_id: i64,
        bounds: HistoryBounds,
        limit: Option<i32>,
        reverse: bool,
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError> {
        let mut query = history_query(user_id, bounds, limit, reverse);
        let mut chapters: Vec<_> = query
            .build()
            .fetch_all(self.pool.read())
            .await?
            .into_iter()
            .map(|row| HistoryChapter {
                manga_id: row.get(0),
                chapter_id: row.get(1),
                manga_title: row.get(2),
                cover_url: row.get(3),
                chapter_title: row.get(4),
                read_at: row.get(5),
                last_page_read: row.get(6),
                is_complete: row.get(7),
                source_id: row.get(8),
            })
            .collect();
        if reverse {
            chapters.reverse();
        }
        Ok(chapters)
    }
}

#[async_trait]
impl HistoryRepository for HistoryRepositoryImpl {
    async fn get_first_history_chapters(
        &self,
        user_id: i64,
        bounds: HistoryBounds,
        first: i32,
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError> {
        self.history_chapters(user_id, bounds, Some(first), false)
            .await
    }

    async fn get_last_history_chapters(
        &self,
        user_id: i64,
        bounds: HistoryBounds,
        last: i32,
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError> {
        self.history_chapters(user_id, bounds, Some(last), true)
            .await
    }

    async fn get_history_chapters(
        &self,
        user_id: i64,
        bounds: HistoryBounds,
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError> {
        self.history_chapters(user_id, bounds, None, false).await
    }

    async fn get_history_page_info(
        &self,
        user_id: i64,
        first: HistoryCursor,
        last: HistoryCursor,
    ) -> Result<HistoryPageInfo, HistoryRepositoryError> {
        let (has_previous_page, has_next_page): (bool, bool) =
            sqlx::query_as(HISTORY_PAGE_INFO_SQL)
                .bind(user_id)
                .bind(history_timestamp(first.read_at))
                .bind(first.manga_id)
                .bind(user_id)
                .bind(history_timestamp(last.read_at))
                .bind(last.manga_id)
                .fetch_one(self.pool.read())
                .await?;
        Ok(HistoryPageInfo {
            has_previous_page,
            has_next_page,
        })
    }

    async fn get_last_read_at_by_manga_ids(
        &self,
        user_id: i64,
        manga_ids: &[i64],
    ) -> Result<HashMap<i64, NaiveDateTime>, HistoryRepositoryError> {
        if manga_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let query_str = format!(
            "SELECT manga_id, read_at FROM user_manga_history \
             WHERE user_id = ? AND manga_id IN ({})",
            vec!["?"; manga_ids.len()].join(",")
        );

        let mut query = sqlx::query_as::<_, (i64, NaiveDateTime)>(&query_str).bind(user_id);

        for manga_id in manga_ids {
            query = query.bind(manga_id);
        }

        let last_read_at = query
            .fetch_all(self.pool.read())
            .await?
            .into_iter()
            .collect();

        Ok(last_read_at)
    }

    async fn get_history_chapters_by_chapter_ids(
        &self,
        user_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<HistoryChapter>, HistoryRepositoryError> {
        let query_str = format!(
            r#"
        SELECT
            manga.id,
            chapter.id,
            manga.title,
            manga.cover_url,
            chapter.title,
            user_history.read_at,
            user_history.last_page,
            user_history.is_complete,
            manga.source_id
        FROM
            user_history
            JOIN chapter 
                ON chapter.id = user_history.chapter_id
            JOIN manga ON manga.id = chapter.manga_id
        WHERE
            user_history.user_id = ?
            AND user_history.chapter_id IN ({})"#,
            vec!["?"; chapter_ids.len()].join(",")
        );

        let mut query = sqlx::query(&query_str).bind(user_id);

        for chapter_id in chapter_ids {
            query = query.bind(chapter_id);
        }

        let chapters = query
            .fetch_all(self.pool.read())
            .await?
            .into_par_iter()
            .map(|row| HistoryChapter {
                manga_id: row.get(0),
                chapter_id: row.get(1),
                manga_title: row.get(2),
                cover_url: row.get(3),
                chapter_title: row.get(4),
                read_at: row.get(5),
                last_page_read: row.get(6),
                is_complete: row.get(7),
                source_id: row.get(8),
            })
            .collect();

        Ok(chapters)
    }

    async fn insert_history_chapter(
        &self,
        user_id: i64,
        chapter_id: i64,
        page: i64,
        is_complete: bool,
    ) -> Result<(), HistoryRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO user_history(user_id, chapter_id, last_page, read_at, is_complete)
            VALUES(?, ?, ?, ?, ?)
            ON CONFLICT(user_id, chapter_id)
            DO UPDATE SET
                last_page = excluded.last_page,
                read_at = excluded.read_at,
                is_complete = CASE is_complete WHEN 0 THEN excluded.is_complete ELSE is_complete END"#,
        )
        .bind(user_id)
        .bind(chapter_id)
        .bind(page)
        .bind(Utc::now().naive_utc())
        .bind(is_complete)
        .execute(self.pool.write())
        .await?;

        Ok(())
    }

    async fn insert_history_chapters_as_completed(
        &self,
        user_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), HistoryRepositoryError> {
        if chapter_ids.is_empty() {
            return Ok(());
        }

        let query_str = format!(
            r#"
            INSERT INTO user_history(user_id, chapter_id, last_page, read_at, is_complete)
            VALUES {}
            ON CONFLICT(user_id, chapter_id)
            DO UPDATE SET
                last_page = excluded.last_page,
                read_at = excluded.read_at,
                is_complete = excluded.is_complete"#,
            vec!["(?, ?, 0, ?, true)"; chapter_ids.len()].join(",")
        );

        let mut query = sqlx::query(&query_str);

        let now = Utc::now().naive_utc();
        for chapter_id in chapter_ids {
            query = query.bind(user_id).bind(chapter_id).bind(now);
        }

        query.execute(self.pool.write()).await?;

        Ok(())
    }

    async fn delete_chapters_from_history(
        &self,
        user_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), HistoryRepositoryError> {
        if chapter_ids.is_empty() {
            return Ok(());
        }

        let mut values = vec![];
        values.resize(chapter_ids.len(), "?");

        let query_str = format!(
            r#"DELETE FROM user_history WHERE user_id = ? AND chapter_id IN ({})"#,
            values.join(",")
        );

        let mut query = sqlx::query(&query_str).bind(user_id);

        for chapter_id in chapter_ids {
            query = query.bind(chapter_id);
        }

        query.execute(self.pool.write()).await?;

        Ok(())
    }

    async fn get_unread_chapters_by_manga_ids(
        &self,
        user_id: i64,
        manga_ids: &[i64],
    ) -> Result<HashMap<i64, i64>, HistoryRepositoryError> {
        let mut values = vec![];
        values.resize(manga_ids.len(), "?");

        let query_str = format!(
            r#"
            SELECT
                manga_id,
                COUNT(1)
            FROM (
                SELECT
                    c.manga_id,
                    IFNULL(user_history.is_complete, false) AS is_complete 
                FROM chapter c 
                    LEFT JOIN user_history
                        ON user_history.user_id = ?
                        AND user_history.chapter_id = c.id 
                WHERE c.manga_id IN ({})
            )
            WHERE is_complete = false
            GROUP BY manga_id"#,
            values.join(",")
        );

        let mut query = sqlx::query(&query_str).bind(user_id);
        for manga_id in manga_ids {
            query = query.bind(manga_id);
        }

        let data = query
            .fetch_all(self.pool.read())
            .await?
            .iter()
            .map(|row| (row.get(0), row.get(1)))
            .collect();

        Ok(data)
    }

    async fn get_next_chapter_by_manga_id(
        &self,
        user_id: i64,
        manga_id: i64,
    ) -> Result<Option<i64>, HistoryRepositoryError> {
        let chapter_id = sqlx::query(
            r#"
            WITH last_reading_session AS (
                SELECT
                    chapter_id,
                    is_complete,
                    number as chapter_number
                FROM
                    user_history
                    INNER JOIN chapter on chapter.id = user_history.chapter_id
                    AND chapter.manga_id = ?
                WHERE
                    user_history.user_id = ?
                ORDER BY
                    user_history.read_at DESC
                LIMIT
                    1
            ), first_unread_chapter AS (
                SELECT
                    id
                FROM
                    chapter
                    LEFT JOIN user_history ON user_history.chapter_id = chapter.id
                    AND user_history.user_id = ?
                WHERE
                    chapter.manga_id = ?
                    AND user_history.is_complete IS NOT true
                ORDER BY
                    chapter.number ASC
                LIMIT
                    1
            ), resume_chapter AS (
                SELECT
                    COALESCE(
                        CASE
                            WHEN is_complete THEN (
                                SELECT
                                    id
                                FROM
                                    chapter
                                    LEFT JOIN user_history ON user_history.chapter_id = chapter.id
                                    AND user_history.user_id = ?
                                WHERE
                                    chapter.number > chapter_number
                                    AND chapter.manga_id = ?
                                    AND user_history.is_complete IS NOT true
                                ORDER BY
                                    number ASC
                                LIMIT
                                    1
                            )
                            ELSE chapter_id
                        END,
                        first_unread_chapter.id
                    ) AS id
                FROM
                    (SELECT null)
                    LEFT JOIN first_unread_chapter
                    LEFT JOIN last_reading_session
            )
            SELECT
                chapter.id
            FROM
                chapter
            WHERE
                chapter.id = (
                    SELECT
                        id
                    FROM
                        resume_chapter
                )"#,
        )
        .bind(manga_id)
        .bind(user_id)
        .bind(user_id)
        .bind(manga_id)
        .bind(user_id)
        .bind(manga_id)
        .fetch_optional(self.pool.read())
        .await?
        .map(|row| row.get(0));

        Ok(chapter_id)
    }
}

// Always bind the fixed-width representation maintained by the migration's
// triggers, including all nine fractional digits. This keeps tuple comparisons
// exact for timestamps SQLite's date functions would round to milliseconds.
fn history_timestamp(value: NaiveDateTime) -> String {
    value.format("%Y-%m-%d %H:%M:%S%.9f").to_string()
}

fn history_query(
    user_id: i64,
    bounds: HistoryBounds,
    limit: Option<i32>,
    reverse: bool,
) -> QueryBuilder<'static, Sqlite> {
    let mut query = QueryBuilder::new(
        "SELECT latest.manga_id, latest.chapter_id, manga.title, manga.cover_url, \
         chapter.title, latest.read_at, history.last_page, history.is_complete, manga.source_id \
         FROM user_manga_history latest \
         JOIN chapter ON chapter.id = latest.chapter_id \
         JOIN manga ON manga.id = latest.manga_id \
         JOIN user_history history ON history.user_id = latest.user_id \
             AND history.chapter_id = latest.chapter_id \
         WHERE latest.user_id = ",
    );
    query.push_bind(user_id);
    if let Some(after) = bounds.after {
        query
            .push(" AND (latest.read_at, latest.manga_id) < (")
            .push_bind(history_timestamp(after.read_at))
            .push(", ")
            .push_bind(after.manga_id)
            .push(")");
    }
    if let Some(before) = bounds.before {
        query
            .push(" AND (latest.read_at, latest.manga_id) > (")
            .push_bind(history_timestamp(before.read_at))
            .push(", ")
            .push_bind(before.manga_id)
            .push(")");
    }
    query.push(if reverse {
        " ORDER BY latest.read_at ASC, latest.manga_id ASC"
    } else {
        " ORDER BY latest.read_at DESC, latest.manga_id DESC"
    });
    if let Some(limit) = limit {
        query.push(" LIMIT ").push_bind(limit);
    }
    query
}

#[cfg(test)]
mod tests;
