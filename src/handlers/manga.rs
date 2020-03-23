use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};
use sled::{Batch, Tree};
use warp::Rejection;

use crate::auth::Claims;
use crate::scraper::{mangasee::Mangasee, GetParams, Params, Scraping};

pub async fn list_mangas(
    source: String,
    param: Params,
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Rejection> {
    if let Ok(url) = get_source_url(source.clone(), db.clone()) {
        let mangas = Mangasee::get_mangas(&url, param);

        let conn = db.lock().unwrap();
        for m in mangas.clone().mangas {
            conn.execute(
                "INSERT OR IGNORE INTO manga(
                    source_id, 
                    title, 
                    path, 
                    thumbnail_url
                    ) VALUES (
                    (SELECT id FROM source WHERE name = ?1), 
                    ?2, 
                    ?3, 
                    ?4)",
                params![source.clone(), m.title, m.path, m.thumbnail_url],
            )
            .unwrap();
        }
        return Ok(warp::reply::json(&mangas));
    }
    Err(warp::reject())
}

pub async fn get_manga_info(
    source: String,
    title: String,
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Rejection> {
    let title = decode_title(title);
    if let Ok(url) = get_manga_url(source.clone(), title.clone(), db.clone()) {
        let manga = Mangasee::get_manga_info(&url);

        let conn = db.lock().unwrap();
        conn.execute(
            "UPDATE manga SET author = ?1, status = ?2, description = ?3
                WHERE manga.source_id = (
                SELECT source.id FROM source
                WHERE source.name = ?4)
                AND manga.title = ?5",
            params![
                manga.manga.author.clone(),
                manga.manga.status.clone(),
                manga.manga.description.clone(),
                source.clone(),
                title.clone()
            ],
        )
        .unwrap();

        return Ok(warp::reply::json(&manga));
    }
    Err(warp::reject())
}

pub async fn get_chapters(
    source: String,
    title: String,
    param: GetParams,
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Rejection> {
    let title = decode_title(title);
    if let Ok(url) = get_manga_url(source.clone(), title.clone(), db.clone()) {
        let chapter = Mangasee::get_chapters(&url);

        let conn = db.lock().unwrap();
        for c in chapter.clone().chapters {
            conn.execute(
                "INSERT OR IGNORE INTO chapter(manga_id, number, path, uploaded)
                VALUES(
                (SELECT manga.id FROM manga 
                JOIN source ON source.id = manga.source_id 
                WHERE source.name = ?1 AND title = ?2 ), 
                ?3, 
                ?4,
                ?5)",
                params![&source, &title, &c.no, &c.url, &c.uploaded],
            )
            .unwrap();
        }
        return Ok(warp::reply::json(&chapter));
    }
    Err(warp::reject())
}

pub async fn get_pages(
    source: String,
    title: String,
    chapter: String,
    param: GetParams,
    db: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, Rejection> {
    let title = decode_title(title);
    if let Ok(url) = get_chapter_url(source.clone(), title.clone(), chapter.clone(), db.clone()) {
        let pages = Mangasee::get_pages(&url);

        let conn = db.lock().unwrap();
        for i in 0..pages.pages.len() {
            conn.execute(
                "INSERT OR IGNORE INTO page(chapter_id, rank, url)
                VALUES(
                (SELECT chapter.id FROM chapter 
                JOIN manga ON manga.id = chapter.manga_id  
                JOIN source ON source.id = manga.source_id
                WHERE source.name = ?1 AND manga.title = ?2 AND chapter.number = ?3),
                ?4,
                ?5)",
                params![source, title, chapter, i as i32, pages.pages[i].clone()],
            )
            .unwrap();
        }
        return Ok(warp::reply::json(&pages));
    }
    Err(warp::reject())
}

fn encode_title(title: String) -> String {
    base64::encode_config(&title, base64::URL_SAFE_NO_PAD)
}

fn decode_title(encoded: String) -> String {
    String::from_utf8(base64::decode_config(encoded, base64::URL_SAFE_NO_PAD).unwrap()).unwrap()
}

fn get_source_url(source: String, db: Arc<Mutex<Connection>>) -> Result<String, String> {
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

fn get_manga_url(
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

fn get_chapter_url(
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
