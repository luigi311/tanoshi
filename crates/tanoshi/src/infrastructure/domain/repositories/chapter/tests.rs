use std::{fs, path::PathBuf};

use chrono::Utc;

use super::ChapterRepositoryImpl;
use crate::{
    domain::{
        entities::chapter::Chapter,
        repositories::{chapter::ChapterRepository, download::DownloadRepository},
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
