use std::{fs, path::PathBuf};

use chrono::{NaiveDateTime, Utc};

use super::ChapterRepositoryImpl;
use crate::{
    domain::{
        entities::chapter::Chapter,
        repositories::{
            chapter::{ChapterRepository, ChapterRepositoryError},
            download::DownloadRepository,
        },
    },
    infrastructure::{
        database::{Pool, establish_connection},
        domain::repositories::download::DownloadRepositoryImpl,
    },
};

struct Fixture {
    dir: PathBuf,
    pool: Pool,
    repo: ChapterRepositoryImpl,
    chapters: Vec<Chapter>,
}

impl Fixture {
    async fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-chapter-upsert-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = establish_connection(dir.join("test.db").to_str().unwrap(), true)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO manga (id, source_id, title, path, cover_url, date_added) \
             VALUES (1, 1, 'Manga 1', '/manga/1', '/cover/1', CURRENT_TIMESTAMP), \
                    (2, 1, 'Manga 2', '/manga/2', '/cover/2', CURRENT_TIMESTAMP)",
        )
        .execute(pool.write())
        .await
        .unwrap();

        let repo = ChapterRepositoryImpl::new(pool.clone());
        let mut chapters: Vec<_> = (1..=2)
            .map(|id| Chapter {
                id: 0,
                source_id: 1,
                manga_id: id,
                title: format!("Chapter {id}"),
                path: format!("/chapter/{id}"),
                number: id as f64,
                scanlator: "Original group".into(),
                uploaded: Utc::now().naive_utc(),
                date_added: Utc::now().naive_utc(),
                downloaded_path: None,
                next: None,
                prev: None,
            })
            .collect();
        repo.insert_chapters(&chapters).await.unwrap();
        // Give the downloads an established order opposite to their insert order.
        sqlx::query(
            "UPDATE chapter SET date_added = CASE manga_id \
                 WHEN 1 THEN '2020-01-02 00:00:00' ELSE '2020-01-01 00:00:00' END, \
                 downloaded_path = '/downloads/' || id || '.cbz'",
        )
        .execute(pool.write())
        .await
        .unwrap();
        for chapter in &mut chapters {
            *chapter = repo
                .get_chapter_by_source_id_path(chapter.source_id, &chapter.path)
                .await
                .unwrap();
        }

        sqlx::query("CREATE TABLE chapter_updates (chapter_id INTEGER NOT NULL)")
            .execute(pool.write())
            .await
            .unwrap();
        sqlx::query(
            "CREATE TRIGGER record_chapter_update AFTER UPDATE ON chapter \
             BEGIN INSERT INTO chapter_updates VALUES (new.id); END",
        )
        .execute(pool.write())
        .await
        .unwrap();

        Self {
            dir,
            pool,
            repo,
            chapters,
        }
    }

    async fn updated_ids(&self) -> Vec<i64> {
        sqlx::query_scalar("SELECT chapter_id FROM chapter_updates ORDER BY rowid")
            .fetch_all(self.pool.read())
            .await
            .unwrap()
    }

    fn new_chapters(&self, count: usize) -> Vec<Chapter> {
        (0..count)
            .map(|index| {
                let mut chapter = self.chapters[0].clone();
                chapter.id = 0;
                chapter.title = format!("New chapter {index}");
                chapter.path = format!("/chapter/new/{index}");
                chapter.number = index as f64 + 3.0;
                chapter.downloaded_path = None;
                chapter
            })
            .collect()
    }

    async fn chapter_dates(&self) -> Vec<(String, NaiveDateTime)> {
        sqlx::query_as("SELECT path, date_added FROM chapter ORDER BY path")
            .fetch_all(self.pool.read())
            .await
            .unwrap()
    }

    async fn downloaded_ids(&self) -> Vec<i64> {
        DownloadRepositoryImpl::new(self.pool.clone())
            .get_first_downloaded_chapters(Utc::now().timestamp() + 1, i64::MAX, 0, 0, 10)
            .await
            .unwrap()
            .into_iter()
            .map(|chapter| chapter.id)
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test]
async fn unchanged_chapters_do_not_update_rows_or_reorder_downloads() {
    let fixture = Fixture::new().await;
    let downloaded_ids = fixture.downloaded_ids().await;
    assert_eq!(
        downloaded_ids,
        fixture
            .chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<Vec<_>>()
    );

    fixture.repo.insert_chapters(&[]).await.unwrap();
    fixture
        .repo
        .insert_chapters(&fixture.chapters)
        .await
        .unwrap();

    assert!(fixture.updated_ids().await.is_empty());
    assert_eq!(fixture.downloaded_ids().await, downloaded_ids);
    for original in &fixture.chapters {
        let stored = fixture.repo.get_chapter_by_id(original.id).await.unwrap();
        assert_eq!(stored.date_added, original.date_added);
        assert_eq!(stored.downloaded_path, original.downloaded_path);
    }
    fixture.pool.close().await;
}

#[tokio::test]
async fn changed_metadata_updates_only_changed_chapters_and_preserves_date_added() {
    let fixture = Fixture::new().await;
    let downloaded_ids = fixture.downloaded_ids().await;
    let original = &fixture.chapters[0];
    let mut chapters = fixture.chapters.clone();

    for field in ["title", "number", "scanlator", "uploaded", "manga_id"] {
        match field {
            "title" => chapters[0].title = "Renamed chapter".into(),
            "number" => chapters[0].number = 3.5,
            "scanlator" => chapters[0].scanlator = "New group".into(),
            "uploaded" => chapters[0].uploaded += chrono::Duration::days(1),
            "manga_id" => chapters[0].manga_id = 2,
            _ => unreachable!(),
        }
        fixture.repo.insert_chapters(&chapters).await.unwrap();
        assert_eq!(fixture.updated_ids().await, vec![original.id], "{field}");

        let stored = fixture.repo.get_chapter_by_id(original.id).await.unwrap();
        assert_eq!(stored.title, chapters[0].title);
        assert_eq!(stored.number, chapters[0].number);
        assert_eq!(stored.scanlator, chapters[0].scanlator);
        assert_eq!(stored.uploaded, chapters[0].uploaded);
        assert_eq!(stored.manga_id, chapters[0].manga_id);
        assert_eq!(stored.date_added, original.date_added);
        assert_eq!(stored.downloaded_path, original.downloaded_path);
        assert_eq!(fixture.downloaded_ids().await, downloaded_ids);

        fixture.repo.insert_chapters(&chapters).await.unwrap();
        assert_eq!(fixture.updated_ids().await, vec![original.id], "{field}");
        sqlx::query("DELETE FROM chapter_updates")
            .execute(fixture.pool.write())
            .await
            .unwrap();
    }
    fixture.pool.close().await;
}

#[tokio::test]
async fn new_chapters_get_date_added_without_updating_existing_chapters() {
    let fixture = Fixture::new().await;
    let mut chapters = fixture.chapters.clone();
    let mut new = chapters[0].clone();
    new.id = 0;
    new.title = "New chapter".into();
    new.path = "/chapter/3".into();
    new.number = 3.0;
    new.downloaded_path = None;
    chapters.push(new);

    let before = Utc::now().naive_utc();
    fixture.repo.insert_chapters(&chapters).await.unwrap();
    let after = Utc::now().naive_utc();
    let stored = fixture
        .repo
        .get_chapter_by_source_id_path(1, "/chapter/3")
        .await
        .unwrap();
    assert!(stored.date_added >= before && stored.date_added <= after);
    assert_eq!(stored.title, "New chapter");
    assert_eq!(stored.number, 3.0);
    assert_eq!(stored.downloaded_path, None);
    assert!(fixture.updated_ids().await.is_empty());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chapter")
        .fetch_one(fixture.pool.read())
        .await
        .unwrap();
    assert_eq!(count, 3);
    fixture.pool.close().await;
}

#[tokio::test]
async fn nullable_metadata_is_replaced_on_refresh() {
    let fixture = Fixture::new().await;
    let original = &fixture.chapters[0];
    sqlx::query("UPDATE chapter SET title = NULL, scanlator = NULL WHERE id = ?")
        .bind(original.id)
        .execute(fixture.pool.write())
        .await
        .unwrap();
    sqlx::query("DELETE FROM chapter_updates")
        .execute(fixture.pool.write())
        .await
        .unwrap();

    fixture
        .repo
        .insert_chapters(&fixture.chapters)
        .await
        .unwrap();

    let stored = fixture.repo.get_chapter_by_id(original.id).await.unwrap();
    assert_eq!(stored.title, original.title);
    assert_eq!(stored.scanlator, original.scanlator);
    assert_eq!(stored.date_added, original.date_added);
    assert_eq!(fixture.updated_ids().await, vec![original.id]);
    fixture.pool.close().await;
}

#[tokio::test]
async fn large_chapter_lists_are_inserted_and_refreshed_across_chunks() {
    let fixture = Fixture::new().await;
    let downloaded_ids = fixture.downloaded_ids().await;
    let mut chapters = fixture.chapters.clone();
    // Exceeds the 32,766-variable limit for one statement and leaves a partial chunk.
    chapters.extend(fixture.new_chapters(5_001));

    fixture.repo.insert_chapters(&chapters).await.unwrap();

    let stored_dates = fixture.chapter_dates().await;
    let mut expected_paths: Vec<_> = chapters
        .iter()
        .map(|chapter| chapter.path.as_str())
        .collect();
    expected_paths.sort_unstable();
    assert_eq!(
        stored_dates
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>(),
        expected_paths
    );
    let mut new_dates = stored_dates
        .iter()
        .filter(|(path, _)| path.starts_with("/chapter/new/"))
        .map(|(_, date_added)| date_added);
    let date_added = new_dates.next().unwrap();
    assert!(new_dates.all(|date| date == date_added));
    assert!(fixture.updated_ids().await.is_empty());
    assert_eq!(fixture.downloaded_ids().await, downloaded_ids);

    fixture.repo.insert_chapters(&chapters).await.unwrap();

    assert!(fixture.updated_ids().await.is_empty());
    assert_eq!(fixture.chapter_dates().await, stored_dates);
    assert_eq!(fixture.downloaded_ids().await, downloaded_ids);
    fixture.pool.close().await;
}

#[tokio::test]
async fn later_chunk_failure_rolls_back_inserts_and_metadata_updates() {
    let fixture = Fixture::new().await;
    let original = &fixture.chapters[0];
    let original_dates = fixture.chapter_dates().await;
    let downloaded_ids = fixture.downloaded_ids().await;
    let mut chapters = fixture.chapters.clone();
    chapters[0].title = "Changed in the first chunk".into();
    chapters.extend(fixture.new_chapters(2_001));
    chapters.last_mut().unwrap().path = "/chapter/failing".into();
    sqlx::query(
        "CREATE TRIGGER reject_chapter_insert BEFORE INSERT ON chapter \
         WHEN new.path = '/chapter/failing' \
         BEGIN SELECT RAISE(ABORT, 'injected later chunk failure'); END",
    )
    .execute(fixture.pool.write())
    .await
    .unwrap();

    let error = fixture.repo.insert_chapters(&chapters).await.unwrap_err();
    assert!(error.to_string().contains("injected later chunk failure"));
    assert_eq!(fixture.chapter_dates().await, original_dates);
    assert!(fixture.updated_ids().await.is_empty());
    assert_eq!(
        fixture
            .repo
            .get_chapter_by_id(original.id)
            .await
            .unwrap()
            .title,
        original.title
    );
    assert_eq!(fixture.downloaded_ids().await, downloaded_ids);

    // The writer remains usable and a retry commits the whole list.
    sqlx::query("DROP TRIGGER reject_chapter_insert")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    fixture.repo.insert_chapters(&chapters).await.unwrap();
    assert_eq!(fixture.chapter_dates().await.len(), chapters.len());
    assert_eq!(fixture.updated_ids().await, vec![original.id]);
    fixture.pool.close().await;
}

#[tokio::test]
async fn empty_path_and_id_lists_are_rejected() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        fixture.repo.get_chapters_not_in_source(1, 1, &[]).await,
        Err(ChapterRepositoryError::BadArgsError(_))
    ));
    assert!(matches!(
        fixture.repo.delete_chapter_by_ids(&[]).await,
        Err(ChapterRepositoryError::BadArgsError(_))
    ));
    fixture.pool.close().await;
}

#[tokio::test]
async fn missing_chapters_respect_the_complete_path_set_and_scope() {
    let fixture = Fixture::new().await;
    let mut paths: Vec<_> = (0..33_001)
        .map(|index| format!("/source/chapter/{index}"))
        .collect();
    paths[0] = fixture.chapters[0].path.clone();
    paths[1_000] = "/source/quoted's/第千章".into();
    paths[20_000] = paths[1_000].clone();

    let mut chapters = fixture.new_chapters(2_001);
    let mut expected_paths: Vec<_> = chapters
        .iter()
        .map(|chapter| chapter.path.clone())
        .collect();
    expected_paths.sort_unstable();
    // Keep chapters whose paths occur at widely separated positions in the input.
    for index in [1_000, 33_000] {
        let mut chapter = fixture.chapters[0].clone();
        chapter.path = paths[index].clone();
        chapters.push(chapter);
    }
    let mut other_source = fixture.chapters[0].clone();
    other_source.source_id = 2;
    other_source.path = "/chapter/other-source".into();
    chapters.push(other_source);
    fixture.repo.insert_chapters(&chapters).await.unwrap();

    let missing = fixture
        .repo
        .get_chapters_not_in_source(1, 1, &paths)
        .await
        .unwrap();
    let mut missing_paths: Vec<_> = missing.iter().map(|chapter| chapter.path.clone()).collect();
    missing_paths.sort_unstable();
    assert_eq!(missing_paths, expected_paths);
    assert!(missing.iter().all(|chapter| chapter.source_id == 1
        && chapter.manga_id == 1
        && chapter.next.is_none()
        && chapter.prev.is_none()));

    paths.extend(missing_paths);
    assert!(
        fixture
            .repo
            .get_chapters_not_in_source(1, 1, &paths)
            .await
            .unwrap()
            .is_empty()
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn large_id_lists_delete_only_requested_chapters() {
    let fixture = Fixture::new().await;
    let original_dates = fixture.chapter_dates().await;
    let downloaded_ids = fixture.downloaded_ids().await;
    fixture
        .repo
        .insert_chapters(&fixture.new_chapters(33_001))
        .await
        .unwrap();
    let mut ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM chapter WHERE path LIKE '/chapter/new/%' ORDER BY id")
            .fetch_all(fixture.pool.read())
            .await
            .unwrap();
    assert_eq!(ids.len(), 33_001);
    // Duplicate and nonexistent IDs must remain harmless across batches.
    ids.push(ids[0]);
    ids.push(i64::MAX);

    fixture.repo.delete_chapter_by_ids(&ids).await.unwrap();

    assert_eq!(fixture.chapter_dates().await, original_dates);
    assert_eq!(fixture.downloaded_ids().await, downloaded_ids);
    fixture.pool.close().await;
}

#[tokio::test]
async fn later_delete_chunk_failure_rolls_back_chapters_and_read_history() {
    let fixture = Fixture::new().await;
    fixture
        .repo
        .insert_chapters(&fixture.new_chapters(2_001))
        .await
        .unwrap();
    let original_dates = fixture.chapter_dates().await;
    let downloaded_ids = fixture.downloaded_ids().await;
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM chapter WHERE manga_id = 1 ORDER BY id")
        .fetch_all(fixture.pool.read())
        .await
        .unwrap();
    sqlx::query("INSERT INTO user (id, username, password) VALUES (1, 'chapter-test', 'unused')")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    sqlx::query("INSERT INTO user_history (user_id, chapter_id, last_page) VALUES (1, ?, 3)")
        .bind(fixture.chapters[0].id)
        .execute(fixture.pool.write())
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER reject_chapter_delete BEFORE DELETE ON chapter \
         WHEN old.path = '/chapter/new/2000' \
         BEGIN SELECT RAISE(ABORT, 'injected later delete failure'); END",
    )
    .execute(fixture.pool.write())
    .await
    .unwrap();

    let error = fixture.repo.delete_chapter_by_ids(&ids).await.unwrap_err();
    assert!(error.to_string().contains("injected later delete failure"));
    assert_eq!(fixture.chapter_dates().await, original_dates);
    assert_eq!(fixture.downloaded_ids().await, downloaded_ids);
    let last_page: i64 = sqlx::query_scalar("SELECT last_page FROM user_history WHERE user_id = 1")
        .fetch_one(fixture.pool.read())
        .await
        .unwrap();
    assert_eq!(last_page, 3);

    sqlx::query("DROP TRIGGER reject_chapter_delete")
        .execute(fixture.pool.write())
        .await
        .unwrap();
    fixture.repo.delete_chapter_by_ids(&ids).await.unwrap();
    assert_eq!(
        fixture.chapter_dates().await,
        vec![(
            fixture.chapters[1].path.clone(),
            fixture.chapters[1].date_added
        )]
    );
    let history_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_history")
        .fetch_one(fixture.pool.read())
        .await
        .unwrap();
    assert_eq!(history_count, 0);
    fixture.pool.close().await;
}
