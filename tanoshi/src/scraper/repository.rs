use sqlx::postgres::PgPool;

use tanoshi::manga::{Chapter, GetChaptersResponse, GetMangaResponse, GetPagesResponse, Manga};

pub async fn get_source_url(source: String, db: PgPool) -> Result<String, sqlx::Error> {
    let ret = sqlx::query!(r#"SELECT url FROM source WHERE name = $1"#, source)
        .fetch_one(&db)
        .await?;

    Ok(ret.url)
}

pub async fn get_manga_url(source: String, title: String, db: PgPool) -> Result<String, sqlx::Error> {
    let ret = sqlx::query!(
        r#"SELECT CONCAT(source.url, manga.path) AS url FROM manga 
            JOIN source ON source.id = manga.source_id 
            WHERE source.name = $1 AND  manga.title = $2"#,
        source,
        title
    )
    .fetch_one(&db)
    .await?;

    Ok(ret.url.unwrap())
}

pub async fn get_chapter_url(
    source: String,
    title: String,
    chapter: String,
    db: PgPool,
) -> Result<String, sqlx::Error> {
    let ret = sqlx::query!(
        "SELECT CONCAT(source.url, chapter.path) AS url FROM chapter
            JOIN manga ON manga.id = chapter.manga_id 
            JOIN source ON source.id = manga.source_id 
            WHERE source.name = $1 AND  manga.title = $2 AND chapter.number = $3",
        source,
        title,
        chapter,
    )
    .fetch_one(&db)
    .await?;

    Ok(ret.url.unwrap())
}

pub async fn get_manga_detail(
    source: String,
    title: String,
    username: String,
    db: PgPool,
) -> Result<GetMangaResponse, sqlx::Error> {
    let manga = sqlx::query_as!(
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
    ).fetch_one(&db).await?;

    Ok(GetMangaResponse {manga})
}

pub async fn get_chapters(
    source: String,
    title: String,
    username: String,
    db: PgPool,
) -> Result<GetChaptersResponse, sqlx::Error> {
    let chapters = sqlx::query_as!(
        Chapter,
        r#"SELECT 
        chapter.number AS no, COALESCE(chapter.title, '') AS title, chapter.path AS url, 
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
    .await?;
    if !chapters.is_empty() {
        Ok(GetChaptersResponse { chapters })
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

pub async fn get_pages(
    source: String,
    title: String,
    chapter: String,
    db: PgPool,
) -> Result<GetPagesResponse, sqlx::Error> {
    let pages = sqlx::query!(
        r#"SELECT 
        page.url
        FROM page
        JOIN chapter ON chapter.id = page.chapter_id
        JOIN manga ON manga.id = chapter.manga_id
        JOIN source ON source.id = manga.source_id
        WHERE source.name = $1 AND manga.title = $2 AND chapter.number = $3
        ORDER BY page.rank"#,
        source,
        title,
        chapter
    )
    .fetch_all(&db)
    .await?;

    if !pages.is_empty() {
        let pages = pages.iter().map(|page| page.url.clone()).collect();
        Ok(GetPagesResponse{pages})
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

pub async fn insert_mangas(source: String, mangas: Vec<Manga>, db: PgPool) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for m in mangas {
        sqlx::query!(
                "INSERT INTO manga(
                    source_id,
                    title,
                    path,
                    thumbnail_url
                    ) VALUES (
                    (SELECT id FROM source WHERE name = $1),
                    $2,
                    $3,
                    $4) ON CONFLICT DO NOTHING",
                source,
                m.title,
                m.path,
                m.thumbnail_url,
            )
            .execute(&mut tx)
            .await?;
    }
    tx.commit().await?;

    Ok(())
}

pub async fn insert_chapters(source: String, title: String, chapters: Vec<Chapter>, db: PgPool) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for c in chapters {
        sqlx::query!(
                "INSERT INTO chapter(manga_id, number, title, path, uploaded)
                VALUES(
                (SELECT manga.id FROM manga
                JOIN source ON source.id = manga.source_id
                WHERE source.name = $1 AND title = $2 ),
                $3,
                $4,
                $5,
                $6) ON CONFLICT DO NOTHING",
                source,
                title,
                c.no,
                c.title,
                c.url,
                c.uploaded,
            )
            .execute(&mut tx)
            .await?;
    }
    tx.commit().await?;

    Ok(())
}

pub async fn insert_pages(source: String, title: String, chapter: String, pages: Vec<String>, db: PgPool) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for i in 0..pages.len() {
        sqlx::query!(
                "INSERT INTO page(chapter_id, rank, url)
                VALUES(
                (SELECT chapter.id FROM chapter
                JOIN manga ON manga.id = chapter.manga_id
                JOIN source ON source.id = manga.source_id
                WHERE source.name = $1 AND manga.title = $2 AND chapter.number = $3),
                $4,
                $5) ON CONFLICT DO NOTHING",
                source,
                title,
                chapter,
                (i as i32),
                pages[i],
            )
            .execute(&mut tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

