use sqlx::postgres::{PgArguments, PgPool, PgQueryAs, PgRow};
use sqlx::{arguments::Arguments, Row};
use tanoshi_lib::manga::{
    Chapter, GetChaptersResponse, GetMangaResponse, GetMangasResponse, GetPagesResponse, Image,
    Manga, Source,
};

pub async fn get_sources(db: PgPool) -> Result<Vec<Source>, sqlx::Error> {
    let sources = sqlx::query_as!(Source, "SELECT id, name, url, version FROM source")
        .fetch_all(&db)
        .await?;
    Ok(sources)
}

pub async fn get_source(source_id: i32, db: PgPool) -> Result<Source, sqlx::Error> {
    let ret = sqlx::query_as!(
        Source,
        r#"SELECT id, name, url, version FROM source WHERE id = $1"#,
        source_id
    )
    .fetch_one(&db)
    .await?;

    Ok(ret)
}

pub async fn get_source_from_manga_id(manga_id: i32, db: PgPool) -> Result<Source, sqlx::Error> {
    let ret = sqlx::query_as!(
        Source,
        r#"SELECT source.id, source.name, source.url, source.version
        FROM manga
        JOIN source ON source.id = manga.source_id 
        WHERE manga.id = $1"#,
        manga_id
    )
    .fetch_one(&db)
    .await?;

    Ok(ret)
}

pub async fn get_image_from_image_url(url: String, db: PgPool) -> Result<Image, sqlx::Error> {
    let ret = sqlx::query_as!(
        Image,
        r#"SELECT 
        s.id AS source_id, 
        s.name AS source_name, 
        s.id::TEXT || '/' || m.id::TEXT || '/' || c.id::TEXT AS path,
        page.rank::TEXT || substring(page.url from '\.[a-zA-Z]+$') AS file_name 
        FROM page
        JOIN chapter c on page.chapter_id = c.id
        JOIN manga m on c.manga_id = m.id
        JOIN source s on m.source_id = s.id
        WHERE page.url = $1"#,
        url
    )
    .fetch_one(&db)
    .await?;

    Ok(ret)
}

pub async fn get_source_from_chapter_id(
    chapter_id: i32,
    db: PgPool,
) -> Result<Source, sqlx::Error> {
    let ret = sqlx::query_as!(
        Source,
        r#"SELECT source.id, source.name, source.url, source.version 
        FROM chapter
        JOIN manga ON manga.id = chapter.manga_id
        JOIN source ON source.id = manga.source_id 
        WHERE chapter.id = $1"#,
        chapter_id
    )
    .fetch_one(&db)
    .await?;

    Ok(ret)
}

pub async fn get_manga_url(manga_id: i32, db: PgPool) -> Result<String, sqlx::Error> {
    let ret = sqlx::query!(
        r#"SELECT CONCAT(source.url, manga.path) AS url FROM manga 
            JOIN source ON source.id = manga.source_id 
            WHERE manga.id = $1"#,
        manga_id
    )
    .fetch_one(&db)
    .await?;

    Ok(ret.url.unwrap())
}

pub async fn get_chapter_url(chapter_id: i32, db: PgPool) -> Result<String, sqlx::Error> {
    let ret = sqlx::query!(
        "SELECT CONCAT(source.url, chapter.path) AS url FROM chapter
            JOIN manga ON manga.id = chapter.manga_id 
            JOIN source ON source.id = manga.source_id 
            WHERE chapter.id = $1",
        chapter_id
    )
    .fetch_one(&db)
    .await?;

    Ok(ret.url.unwrap())
}

pub async fn get_mangas(
    username: String,
    manga_ids: Vec<i32>,
    db: PgPool,
) -> Result<GetMangasResponse, sqlx::Error> {
    let mut i: i32 = 1;
    let id_param = manga_ids
        .iter()
        .map(|_| {
            i += 1;
            format!("${}", i)
        })
        .collect::<Vec<String>>()
        .join(", ");
    let mut args = PgArguments::default();
    args.add(username);
    for id in manga_ids {
        args.add(id);
    }
    let query_str = format!(
        r#"SELECT
    manga.id AS id,
    manga.title AS title,
    CASE
        WHEN author IS NOT NULL THEN author
        ELSE ''
        END author,
    CASE
        WHEN status IS NOT NULL THEN status
        ELSE ''
        END status,
    CASE
        WHEN description IS NOT NULL THEN description
        ELSE ''
        END description,
    manga.path,
    thumbnail_url,
    CASE WHEN f.manga_id IS NULL
         THEN false
         ELSE
             true
     END is_favorite
     FROM manga
     LEFT JOIN favorite f on manga.id = f.manga_id AND f.user_id = (SELECT id FROM "user" WHERE username = $1)
     LEFT JOIN unnest(ARRAY[{}]) WITH ORDINALITY AS m(id, rn) ON manga.id = m.id
     WHERE manga.id IN ({}) ORDER BY m.rn"#,
        id_param, id_param
    );
    let mangas = sqlx::query(&query_str)
        .bind_all(args)
        .map(|row: PgRow| Manga {
            id: row.get(0),
            title: row.get(1),
            author: row.get(2),
            status: row.get(3),
            description: row.get(4),
            path: row.get(5),
            thumbnail_url: row.get(6),
            last_read: None,
            last_page: None,
            is_favorite: row.get(7),
        })
        .fetch_all(&db)
        .await?;

    Ok(GetMangasResponse { mangas: mangas })
}

pub async fn get_manga_detail(
    manga_id: i32,
    username: String,
    db: PgPool,
) -> Result<GetMangaResponse, sqlx::Error> {
    let manga = sqlx::query_as!(
        Manga,
        r#"SELECT
        manga.id,
       manga.title AS title,
       author,
       status,
       description,
       manga.path,
       thumbnail_url,
       h.chapter_id AS last_read,
       h.last_page,
       CASE WHEN f.manga_id IS NULL
            THEN false
            ELSE
                true
        END is_favorite
        FROM manga
        LEFT JOIN favorite f on manga.id = f.manga_id AND f.user_id = (SELECT id FROM "user" WHERE username = $1)
        LEFT JOIN (
            SELECT chapter.manga_id, history.chapter_id, history.last_page, MAX(history.updated) FROM chapter
            JOIN manga ON chapter.manga_id = manga.id
            JOIN history ON history.chapter_id = chapter.id AND history.user_id = (SELECT id FROM "user" WHERE username = $1)
            GROUP BY chapter.manga_id, history.chapter_id, history.last_page
            ) h ON h.manga_id = manga.id
        WHERE manga.id = $2"#,
        username, manga_id,
    ).fetch_one(&db).await?;

    Ok(GetMangaResponse { manga })
}

pub async fn get_chapters(
    manga_id: i32,
    username: String,
    db: PgPool,
) -> Result<GetChaptersResponse, sqlx::Error> {
    let chapters = sqlx::query_as!(
        Chapter,
        r#"SELECT 
        chapter.id,
        chapter.manga_id,
        chapter.number AS no, COALESCE(chapter.title, '') AS title, chapter.path AS url, 
        COALESCE(history.last_page, 0) AS read,
        chapter.uploaded AS uploaded
        FROM chapter
        LEFT JOIN history ON chapter.id = history.chapter_id
        AND history.user_id = (SELECT id FROM "user" WHERE username = $1)
        WHERE chapter.manga_id = $2
        ORDER BY CAST((CASE
            WHEN chapter.number = '' IS TRUE THEN '0'
            ELSE chapter.number
            END) AS DECIMAL) DESC"#,
        username,
        manga_id
    )
    .fetch_all(&db)
    .await?;
    if !chapters.is_empty() {
        Ok(GetChaptersResponse { chapters })
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

pub async fn get_pages(chapter_id: i32, db: PgPool) -> Result<GetPagesResponse, sqlx::Error> {
    let manga_id: (i32,) = sqlx::query_as(
        r#"SELECT manga.id
        FROM chapter
        JOIN manga ON manga.id = chapter.manga_id
        WHERE chapter.id = $1"#,
    )
    .bind(chapter_id)
    .fetch_one(&db)
    .await?;

    let pages: Vec<String> = sqlx::query(
        r#"SELECT 
        CONCAT('/api/image?url=', page.url) AS url
        FROM page
        WHERE page.chapter_id = $1
        ORDER BY page.rank"#,
    )
    .bind(chapter_id)
    .map(|row: PgRow| row.get(0))
    .fetch_all(&db)
    .await?;

    if !pages.is_empty() {
        Ok(GetPagesResponse {
            manga_id: manga_id.0,
            pages,
        })
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

pub async fn insert_sources(sources: Vec<Source>, db: PgPool) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for source in sources {
        sqlx::query!(
            r"INSERT INTO source(name, url, version) VALUES ($1, $2, $3) ON CONFLICT(name) DO UPDATE SET url=EXCLUDED.url",
            source.name,
            source.url,
            source.version
        )
        .execute(&mut tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

pub async fn insert_mangas(
    source_id: i32,
    mangas: Vec<Manga>,
    db: PgPool,
) -> Result<Vec<i32>, sqlx::Error> {
    let mut ids = vec![];
    let mut tx = db.begin().await?;
    for m in mangas {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO manga(source_id, title, path, thumbnail_url) 
                VALUES ($1, $2, $3, $4) 
                ON CONFLICT(source_id, path) DO UPDATE SET thumbnail_url=EXCLUDED.thumbnail_url
                RETURNING id",
        )
        .bind(source_id)
        .bind(m.title)
        .bind(m.path)
        .bind(m.thumbnail_url)
        .fetch_one(&mut tx)
        .await?;
        ids.push(row.0);
    }
    tx.commit().await?;

    Ok(ids)
}

pub async fn insert_chapters(
    manga_id: i32,
    chapters: Vec<Chapter>,
    db: PgPool,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for c in chapters {
        sqlx::query!(
            "INSERT INTO chapter(manga_id, number, title, path, uploaded)
                VALUES(
                $1,
                $2,
                $3,
                $4,
                $5) ON CONFLICT DO NOTHING",
            manga_id,
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

pub async fn insert_pages(
    chapter_id: i32,
    pages: Vec<String>,
    db: PgPool,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for i in 0..pages.len() {
        sqlx::query!(
            "INSERT INTO page(chapter_id, rank, url)
                VALUES(
                $1,
                $2,
                $3) ON CONFLICT DO NOTHING",
            chapter_id,
            (i as i32),
            pages[i],
        )
        .execute(&mut tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn update_manga_info(manga_id: i32, manga: Manga, db: PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE manga SET author = $1, status = $2, description = $3 WHERE manga.id = $4",
        manga.author,
        manga.status,
        manga.description,
        manga_id,
    )
    .execute(&db)
    .await?;

    Ok(())
}
