//! Dummy queue entries for manually reproducing request contention in the app.
//! No manga, chapter, library, or downloaded-file records are created.

use super::DownloadRepositoryImpl;

const SOURCE_NAME: &str = "Tanoshi queue repro";

fn manga_title(run_id: &str) -> anyhow::Result<String> {
    anyhow::ensure!(
        run_id.len() == 32 && run_id.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Queue test run ID must contain 32 hexadecimal characters"
    );
    Ok(format!("Queue repro - {run_id}"))
}

impl DownloadRepositoryImpl {
    pub async fn seed_queue_repro(
        &self,
        run_id: &str,
        chapters: i64,
        pages_per_chapter: i64,
    ) -> anyhow::Result<Vec<i64>> {
        let title = manga_title(run_id)?;
        anyhow::ensure!(
            (1..=5_000).contains(&chapters),
            "Chapter count must be between 1 and 5000"
        );
        anyhow::ensure!(
            (1..=100).contains(&pages_per_chapter),
            "Page count must be between 1 and 100"
        );

        // Reserve IDs and priorities atomically, including simultaneous seed requests.
        let mut tx = self.pool.write().begin_with("BEGIN IMMEDIATE").await?;
        let existing: Vec<i64> = sqlx::query_scalar(
            "SELECT chapter_id FROM download_queue \
             WHERE source_id = -1 AND source_name = ? AND manga_title = ? AND chapter_id < 0 \
             GROUP BY chapter_id ORDER BY priority, chapter_id",
        )
        .bind(SOURCE_NAME)
        .bind(&title)
        .fetch_all(&mut *tx)
        .await?;
        // Retrying after a lost response must not create another batch.
        if !existing.is_empty() {
            tx.commit().await?;
            return Ok(existing);
        }

        let minimum: Option<i64> = sqlx::query_scalar(
            "SELECT MIN(id) FROM (SELECT MIN(chapter_id) AS id FROM download_queue \
             UNION ALL SELECT MIN(id) FROM chapter)",
        )
        .fetch_one(&mut *tx)
        .await?;
        let first_id = minimum
            .unwrap_or(0)
            .min(0)
            .checked_sub(1)
            .ok_or_else(|| anyhow::anyhow!("No dummy chapter IDs available"))?;
        anyhow::ensure!(
            first_id as i128 - chapters as i128 >= -9_007_199_254_740_991,
            "Dummy chapter IDs exceed JavaScript's safe integer range"
        );
        let last_priority: Option<i64> =
            sqlx::query_scalar("SELECT MAX(priority) FROM download_queue")
                .fetch_one(&mut *tx)
                .await?;
        let priority = last_priority
            .unwrap_or(-1)
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Queue priority overflow"))?;
        anyhow::ensure!(
            priority.checked_add(chapters).is_some(),
            "Queue priority overflow"
        );

        // Append the test batch so cancellations leave existing queue priorities alone.
        sqlx::query(
            "WITH RECURSIVE chapters(n) AS (SELECT 0 UNION ALL SELECT n + 1 FROM chapters WHERE n + 1 < ?), \
             pages(rank) AS (SELECT 0 UNION ALL SELECT rank + 1 FROM pages WHERE rank + 1 < ?) \
             INSERT INTO download_queue \
             (source_id, source_name, manga_id, manga_title, chapter_id, chapter_title, rank, url, priority, date_added) \
             SELECT -1, ?, ?, ?, ? - n, 'Dummy chapter ' || (n + 1), rank, \
             'https://queue-repro.invalid/' || ? || '/' || n || '/' || rank || '.jpg', ? + n, unixepoch() \
             FROM chapters CROSS JOIN pages",
        )
        .bind(chapters)
        .bind(pages_per_chapter)
        .bind(SOURCE_NAME)
        .bind(first_id)
        .bind(&title)
        .bind(first_id)
        .bind(run_id)
        .bind(priority)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((0..chapters).map(|index| first_id - index).collect())
    }

    pub async fn clear_queue_repro(&self, run_id: &str) -> anyhow::Result<i64> {
        let title = manga_title(run_id)?;
        let mut tx = self.pool.write().begin_with("BEGIN IMMEDIATE").await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT chapter_id) FROM download_queue \
             WHERE source_id = -1 AND source_name = ? AND manga_title = ? AND chapter_id < 0",
        )
        .bind(SOURCE_NAME)
        .bind(&title)
        .fetch_one(&mut *tx)
        .await?;
        // Cleanup is a single bulk delete; the stress test uses ordinary removals.
        sqlx::query(
            "DELETE FROM download_queue \
             WHERE source_id = -1 AND source_name = ? AND manga_title = ? AND chapter_id < 0",
        )
        .bind(SOURCE_NAME)
        .bind(title)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(count)
    }
}
