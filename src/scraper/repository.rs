use std::sync::{Arc, Mutex};

use postgres::Client;

use crate::scraper::{Chapter, GetChaptersResponse, GetMangaResponse, GetMangasResponse, Manga};

pub fn get_source_url(source: String, db: Arc<Mutex<Client>>) -> Result<String, String> {
    let mut conn = db.lock().unwrap();
    match conn.query_one("SELECT url FROM source WHERE name = ?1", &[&source]) {
        Ok(row) => Ok(row.get(0)),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_manga_url(
    source: String,
    title: String,
    db: Arc<Mutex<Client>>,
) -> Result<String, String> {
    let mut conn = db.lock().unwrap();
    match conn.query_one(
        "SELECT source.url || manga.path FROM manga 
            JOIN source ON source.id = manga.source_id 
            WHERE source.name = ?1 AND  manga.title = ?2",
        &[&source, &title],
    ) {
        Ok(row) => Ok(row.get(0)),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_chapter_url(
    source: String,
    title: String,
    chapter: String,
    db: Arc<Mutex<Client>>,
) -> Result<String, String> {
    let mut conn = db.lock().unwrap();
    match conn.query_one(
        "SELECT source.url || chapter.path FROM chapter
            JOIN manga ON manga.id = chapter.manga_id 
            JOIN source ON source.id = manga.source_id 
            WHERE source.name = ?1 AND  manga.title = ?2 AND chapter.number = ?3",
        &[&source, &title, &chapter],
    ) {
        Ok(row) => Ok(row.get(0)),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_manga_detail(
    source: String,
    title: String,
    username: String,
    db: Arc<Mutex<Client>>,
) -> Result<GetMangaResponse, String> {
    let mut conn = db.lock().unwrap();
    match conn.query_one(
        "SELECT
       manga.title,
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
        JOIN source ON source.id = manga.source_id AND source.name = ?1
        LEFT JOIN favorite f on manga.id = f.manga_id AND f.user_id = (SELECT id FROM user WHERE username = ?2)
        LEFT JOIN (
            SELECT chapter.manga_id, chapter.number, history.last_page, MAX(history.updated) FROM chapter
            JOIN manga ON chapter.manga_id = manga.id
            JOIN history ON history.chapter_id = chapter.id AND history.user_id = (SELECT id FROM user WHERE username = ?2)
            ) h ON h.manga_id = manga.id
        WHERE manga.title = ?3",
        &[&source, &username, &title],
    ) {
        Ok(row) => Ok(GetMangaResponse {
            manga: Manga {
                title: row.get(0),
                author: row.get(1),
                genre: vec![],
                status: row.get(2),
                description: row.get(3),
                path: row.get(4),
                thumbnail_url: row.get(5),
                last_read: row.get(6),
                last_page: row.get(7),
                is_favorite: row.get(8),
            }
        }),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_chapters(
    source: String,
    title: String,
    username: String,
    db: Arc<Mutex<Client>>,
) -> Result<GetChaptersResponse, String> {
    let mut conn = db.lock().unwrap();
    let stmt = conn
        .prepare(
            "SELECT 
                chapter.number, chapter.path, 
                IFNULL(history.last_page, 0) as last_page,
                chapter.uploaded
                FROM chapter
                JOIN manga ON manga.id = chapter.manga_id
                JOIN source ON source.id = manga.source_id
                LEFT JOIN history ON chapter.id = history.chapter_id
                AND history.user_id = (SELECT id FROM user WHERE username = ?1)
                WHERE source.name = ?2 AND manga.title = ?3
                ORDER BY CAST(chapter.number AS DECIMAL) DESC",
        )
        .unwrap();
    let rows = conn.query(&stmt, &[&username, &source, &title]).unwrap();

    let chapters = rows
        .iter()
        .map(|row| Chapter {
            no: row.get(0),
            url: row.get(1),
            read: row.get(2),
            uploaded: row.get(3),
        })
        .collect::<Vec<Chapter>>();
    if chapters.is_empty() {
        return Err("no chapters".to_string());
    }

    Ok(GetChaptersResponse { chapters: chapters })
}
