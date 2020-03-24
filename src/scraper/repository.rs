use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use crate::scraper::{Chapter, GetChaptersResponse, GetMangaResponse, GetMangasResponse, Manga};

pub fn get_source_url(source: String, db: Arc<Mutex<Connection>>) -> Result<String, String> {
    let conn = db.lock().unwrap();
    match conn.query_row(
        "SELECT url FROM source WHERE name = ?1",
        params![source],
        |row| row.get(0),
    ) {
        Ok(url) => Ok(url),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_manga_url(
    source: String,
    title: String,
    db: Arc<Mutex<Connection>>,
) -> Result<String, String> {
    let conn = db.lock().unwrap();
    match conn.query_row(
        "SELECT source.url || manga.path FROM manga 
            JOIN source ON source.id = manga.source_id 
            WHERE source.name = ?1 AND  manga.title = ?2",
        params![source, title],
        |row| row.get(0),
    ) {
        Ok(url) => Ok(url),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_chapter_url(
    source: String,
    title: String,
    chapter: String,
    db: Arc<Mutex<Connection>>,
) -> Result<String, String> {
    let conn = db.lock().unwrap();
    match conn.query_row(
        "SELECT source.url || chapter.path FROM chapter
            JOIN manga ON manga.id = chapter.manga_id 
            JOIN source ON source.id = manga.source_id 
            WHERE source.name = ?1 AND  manga.title = ?2 AND chapter.number = ?3",
        params![source, title, chapter],
        |row| row.get(0),
    ) {
        Ok(url) => Ok(url),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_manga_detail(
    source: String,
    title: String,
    db: Arc<Mutex<Connection>>,
) -> Result<GetMangaResponse, String> {
    let conn = db.lock().unwrap();
    match conn.query_row(
        "SELECT title, author, status, description, path, thumbnail_url
        JOIN source ON source.id = manga.source_id
        WHERE source.name = ?1 AND ?2",
        params![source, title],
        |row| {
            Ok(Manga {
                title: row.get(0)?,
                author: row.get(1)?,
                genre: vec![],
                status: row.get(2)?,
                description: row.get(3)?,
                path: row.get(4)?,
                thumbnail_url: row.get(5)?,
            })
        },
    ) {
        Ok(m) => Ok(GetMangaResponse { manga: m }),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_chapters(
    source: String,
    title: String,
    username: String,
    db: Arc<Mutex<Connection>>,
) -> Result<GetChaptersResponse, String> {
    let conn = db.lock().unwrap();
    let mut stmt = conn
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
    let chapters_iter = stmt
        .query_map(params![username, source, title], |row| {
            Ok(Chapter {
                no: row.get(0)?,
                url: row.get(1)?,
                read: row.get(2)?,
                uploaded: row.get(3)?,
            })
        })
        .unwrap();

    let mut chapters = vec![];
    for chapter in chapters_iter {
        chapters.push(chapter.unwrap());
    }
    if chapters.is_empty() {
        return Err("no chapters".to_string());
    }

    Ok(GetChaptersResponse { chapters: chapters })
}
