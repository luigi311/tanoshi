use std::{fs, path::PathBuf, time::Instant};

use sqlx::Execute;

use super::*;

struct Fixture {
    dir: PathBuf,
    pool: Pool,
    repo: HistoryRepositoryImpl,
}

impl Fixture {
    async fn new(chapters: i64, chapters_per_manga: i64) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-history-index-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = crate::infrastructure::database::establish_connection(
            dir.join("test.db").to_str().unwrap(),
            true,
        )
        .await
        .unwrap();
        sqlx::query("INSERT INTO user(id, username, password) VALUES(1, 'reader', 'unused')")
            .execute(pool.write())
            .await
            .unwrap();
        sqlx::query(
            r#"
            WITH RECURSIVE n(id) AS (VALUES(1) UNION ALL SELECT id + 1 FROM n WHERE id < ?)
            INSERT INTO manga(id, source_id, title, path, cover_url, date_added)
            SELECT id, 1, 'Manga ' || id, '/manga/' || id, '', CURRENT_TIMESTAMP FROM n
        "#,
        )
        .bind(chapters / chapters_per_manga)
        .execute(pool.write())
        .await
        .unwrap();
        sqlx::query(
            r#"
            WITH RECURSIVE n(id) AS (VALUES(1) UNION ALL SELECT id + 1 FROM n WHERE id < ?)
            INSERT INTO chapter(id, source_id, manga_id, title, path, number, uploaded, date_added)
            SELECT id, 1, (id - 1) / ? + 1, 'Chapter ' || id, '/chapter/' || id, id,
                CURRENT_TIMESTAMP, CURRENT_TIMESTAMP FROM n
        "#,
        )
        .bind(chapters)
        .bind(chapters_per_manga)
        .execute(pool.write())
        .await
        .unwrap();
        sqlx::query(
            r#"
            INSERT INTO user_history(user_id, chapter_id, last_page, read_at, is_complete)
            SELECT 1, id, 5, datetime('2026-01-01', '+' || id || ' seconds'), true FROM chapter
        "#,
        )
        .execute(pool.write())
        .await
        .unwrap();
        let repo = HistoryRepositoryImpl::new(pool.clone());
        Self { dir, pool, repo }
    }

    async fn close(self) {
        self.pool.close().await;
        fs::remove_dir_all(&self.dir).unwrap();
    }
}

#[tokio::test]
async fn history_pages_and_existence_checks_seek_the_summary_index() {
    let f = Fixture::new(100, 10).await;
    let rows = f
        .repo
        .get_history_chapters(1, HistoryBounds::default())
        .await
        .unwrap();
    let cursor = |index: usize| HistoryCursor {
        read_at: rows[index].read_at,
        manga_id: rows[index].manga_id,
    };
    let cases = [
        HistoryBounds::default(),
        HistoryBounds {
            after: Some(cursor(3)),
            before: None,
        },
        HistoryBounds {
            after: None,
            before: Some(cursor(7)),
        },
        HistoryBounds {
            after: Some(cursor(3)),
            before: Some(cursor(7)),
        },
    ];
    for bounds in cases {
        for reverse in [false, true] {
            let mut builder = history_query(1, bounds, Some(2), reverse);
            let mut query = builder.build();
            let sql = format!("EXPLAIN QUERY PLAN {}", query.sql());
            let arguments = query.take_arguments().unwrap().unwrap();
            let plan: Vec<String> = sqlx::query_with(&sql, arguments)
                .fetch_all(f.pool.read())
                .await
                .unwrap()
                .iter()
                .map(|row| row.get(3))
                .collect();
            assert!(
                plan.iter().any(|line| line
                    .contains("SEARCH latest USING INDEX idx_user_manga_history_read_at_manga")),
                "{plan:?}"
            );
            assert!(
                !plan
                    .iter()
                    .any(|line| line.contains("TEMP B-TREE") || line.contains("SCAN history")),
                "{plan:?}"
            );
        }
    }
    let plan: Vec<String> = sqlx::query(&format!("EXPLAIN QUERY PLAN {HISTORY_PAGE_INFO_SQL}"))
        .bind(1)
        .bind(history_timestamp(cursor(3).read_at))
        .bind(cursor(3).manga_id)
        .bind(1)
        .bind(history_timestamp(cursor(7).read_at))
        .bind(cursor(7).manga_id)
        .fetch_all(f.pool.read())
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(3))
        .collect();
    assert_eq!(plan.iter().filter(|line| line.contains("SEARCH user_manga_history USING COVERING INDEX idx_user_manga_history_read_at_manga")).count(), 2, "{plan:?}");
    assert!(
        !plan
            .iter()
            .any(|line| line.contains("user_history") || line.contains("TEMP B-TREE")),
        "{plan:?}"
    );
    f.close().await;
}

#[tokio::test]
#[ignore = "250,000-entry benchmark; run explicitly with --release --ignored --nocapture"]
async fn history_pagination_large_history_benchmark() {
    let f = Fixture::new(250_000, 500).await;
    // The previous production query, measured on the same database and engine.
    const ORIGINAL: &str = r#"
        SELECT manga.id, chapter.id, manga.title, manga.cover_url, chapter.title,
            MAX(user_history.read_at) AS read_at, user_history.last_page,
            user_history.is_complete, manga.source_id
        FROM user_history
        JOIN chapter ON user_history.user_id = ? AND chapter.id = user_history.chapter_id
        JOIN manga ON manga.id = chapter.manga_id
        GROUP BY manga.id
        HAVING read_at < datetime(?, 'unixepoch') AND read_at > datetime(?, 'unixepoch')
        ORDER BY user_history.read_at DESC, manga.id DESC LIMIT ?
    "#;
    let mut report =
        serde_json::json!({ "history_entries": 250_000, "manga": 500, "iterations": 5 });
    for limit in [20, 1] {
        let mut original = vec![];
        let mut indexed = vec![];
        for _ in 0..5 {
            let started = Instant::now();
            let before = sqlx::query(ORIGINAL)
                .bind(1)
                .bind(1_800_000_000_i64)
                .bind(0)
                .bind(limit)
                .fetch_all(f.pool.read())
                .await
                .unwrap();
            original.push(started.elapsed().as_secs_f64() * 1000.0);
            let started = Instant::now();
            let after = f
                .repo
                .get_first_history_chapters(1, HistoryBounds::default(), limit)
                .await
                .unwrap();
            indexed.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(
                before
                    .iter()
                    .map(|row| row.get::<i64, _>(0))
                    .collect::<Vec<_>>(),
                after.iter().map(|row| row.manga_id).collect::<Vec<_>>()
            );
        }
        original.sort_by(f64::total_cmp);
        indexed.sort_by(f64::total_cmp);
        report[format!("limit_{limit}")] =
            serde_json::json!({ "before_median_ms": original[2], "after_median_ms": indexed[2] });
    }
    let rows = f
        .repo
        .get_first_history_chapters(1, HistoryBounds::default(), 20)
        .await
        .unwrap();
    let first = HistoryCursor {
        read_at: rows[0].read_at,
        manga_id: rows[0].manga_id,
    };
    let last = HistoryCursor {
        read_at: rows[19].read_at,
        manga_id: rows[19].manga_id,
    };
    let mut measurements = vec![];
    for _ in 0..5 {
        let started = Instant::now();
        assert_eq!(
            f.repo.get_history_page_info(1, first, last).await.unwrap(),
            HistoryPageInfo {
                has_previous_page: false,
                has_next_page: true
            }
        );
        measurements.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    measurements.sort_by(f64::total_cmp);
    report["both_page_info_checks_median_ms"] = measurements[2].into();
    let output = serde_json::to_string_pretty(&report).unwrap();
    if let Ok(path) = std::env::var("TANOSHI_HISTORY_TEST_REPORT") {
        fs::write(path, &output).unwrap();
    }
    println!("{output}");
    f.close().await;
}
