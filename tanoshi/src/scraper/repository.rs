use std::sync::{Arc, Mutex};

use sqlx::postgres::PgPool;

use crate::scraper::{Chapter, GetChaptersResponse, GetMangaResponse, GetMangasResponse, Manga};

pub async fn get_source_url(source: String, db: PgPool) -> Result<String, String> {
    match sqlx::query!(r#"SELECT url FROM source WHERE name = $1"#, source)
        .fetch_one(&db)
        .await
    {
        Ok(ret) => Ok(ret.url),
        Err(e) => Err(e.to_string()),
    }
}

pub async fn get_manga_url(source: String, title: String, db: PgPool) -> Result<String, String> {
    match sqlx::query!(
        r#"SELECT CONCAT(source.url, manga.path) AS url FROM manga 
            JOIN source ON source.id = manga.source_id 
            WHERE source.name = $1 AND  manga.title = $2"#,
        source,
        title
    )
    .fetch_one(&db)
    .await
    {
        Ok(v) => Ok(v.url.unwrap()),
        Err(e) => Err(e.to_string()),
    }
}

pub async fn get_chapter_url(
    source: String,
    title: String,
    chapter: String,
    db: PgPool,
) -> Result<String, String> {
    match sqlx::query!(
        "SELECT CONCAT(source.url, chapter.path) AS url FROM chapter
            JOIN manga ON manga.id = chapter.manga_id 
            JOIN source ON source.id = manga.source_id 
            WHERE source.name = $1 AND  manga.title = $2 AND chapter.number = $3",
        source,
        title,
        chapter,
    )
    .fetch_one(&db)
    .await
    {
        Ok(ret) => Ok(ret.url.unwrap()),
        Err(e) => Err(e.to_string()),
    }
}

pub async fn get_manga_detail(
    source: String,
    title: String,
    username: String,
    db: PgPool,
) -> Result<GetMangaResponse, String> {
    match sqlx::query_as!(
        Manga,
        r#"SELECT
       manga.title AS title,
       author,
       status,
       description,
       manga.path,
       thumbnail_url,
       h.number AS last_read,
       h.last_page,
       CASE WHEN f.manga_id IS NULL
            THEN false
            ELSE
                true
        END is_favorite
        FROM manga
        JOIN source ON source.id = manga.source_id AND source.name = $1
        LEFT JOIN favorite f on manga.id = f.manga_id AND f.user_id = (SELECT id FROM "user" WHERE username = $2)
        LEFT JOIN (
            SELECT chapter.manga_id, chapter.number, history.last_page, MAX(history.updated) FROM chapter
            JOIN manga ON chapter.manga_id = manga.id
            JOIN history ON history.chapter_id = chapter.id AND history.user_id = (SELECT id FROM "user" WHERE username = $2)
            GROUP BY chapter.manga_id, chapter.number, history.last_page
            ) h ON h.manga_id = manga.id
        WHERE manga.title = $3"#,
        source, username, title,
    ).fetch_one(&db).await {
        Ok(row) => Ok(GetMangaResponse {manga: row}),
        Err(e) => Err(e.to_string()),
    }
}

pub async fn get_chapters(
    source: String,
    title: String,
    username: String,
    db: PgPool,
) -> Result<GetChaptersResponse, String> {
    match sqlx::query_as!(
        Chapter,
        r#"SELECT 
                chapter.number AS no, chapter.path AS url, 
                COALESCE(history.last_page, 0) AS read,
                chapter.uploaded AS uploaded
                FROM chapter
                JOIN manga ON manga.id = chapter.manga_id
                JOIN source ON source.id = manga.source_id
                LEFT JOIN history ON chapter.id = history.chapter_id
                AND history.user_id = (SELECT id FROM "user" WHERE username = $1)
                WHERE source.name = $2 AND manga.title = $3
                ORDER BY CAST(chapter.number AS DECIMAL) DESC"#,
        username,
        source,
        title
    )
    .fetch_all(&db)
    .await
    {
        Ok(chapters) => if chapters.is_empty() {
            Err("not found".to_string())
        } else {
            Ok(GetChaptersResponse { chapters })
        },
        Err(e) => Err(e.to_string()),
    }
}
