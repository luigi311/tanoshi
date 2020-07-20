use rusqlite::types::Value;
use rusqlite::vtab::array;
use rusqlite::{params, Connection};
use tanoshi_lib::manga::{Chapter, Manga};
use tanoshi_lib::rest::{
    GetChaptersResponse, GetMangaResponse, GetMangasResponse, GetPagesResponse,
};

use std::rc::Rc;

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Repository {
    database_path: String,
}

impl Repository {
    pub fn new(database_path: String) -> Self {
        Self { database_path }
    }

    fn connect_db(&self) -> Connection {
        Connection::open(self.database_path.clone()).unwrap()
    }

    pub fn get_manga(&self, manga_id: i32) -> Result<Manga, rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            r#"SELECT id, source, title, author, status, description, path, thumbnail_url FROM manga WHERE manga.id = ?1"#,
            params![manga_id],
            |row| {
                let author = row
                    .get::<_, String>(3)?
                    .split(",")
                    .map(|a| a.to_string())
                    .collect();
                Ok(Manga {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    title: row.get(2)?,
                    author,
                    genre: vec![],
                    status: row.get(4)?,
                    description: row.get(5)?,
                    path: row.get(6)?,
                    thumbnail_url: row.get(7)?,
                    last_read: None,
                    last_page: None,
                    is_favorite: false,
                })
            }
        )?;

        Ok(ret)
    }

    pub fn get_chapter(&self, chapter_id: i32) -> Result<Chapter, rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            "SELECT id, source, manga_id, volume, number, title, path, uploaded FROM chapter WHERE chapter.id = ?1",
            params![chapter_id],
            |row| Ok(Chapter{
                id: row.get(0)?,
                source: row.get(1)?,
                manga_id: row.get(2)?,
                vol: row.get(3)?,
                no: row.get(4)?,
                title: row.get(5)?,
                path: row.get(6)?,
                read: None,
                uploaded: row.get(7)?
            }),
        )?;

        Ok(ret)
    }

    pub fn get_mangas(
        &self,
        username: String,
        manga_ids: Vec<i32>,
    ) -> Result<GetMangasResponse, rusqlite::Error> {
        if manga_ids.is_empty() {
            return Ok(GetMangasResponse {
                mangas: vec![],
                status: "Empty".to_string(),
            });
        }
        let db = self.connect_db();
        array::load_module(&db)?;
        let mut order = manga_ids
            .clone()
            .iter()
            .enumerate()
            .map(|(index, value)| format!("WHEN {} THEN {}\n", value, index))
            .collect::<String>();
        order.push_str("END");

        let mut stmt = db.prepare(
            format!(r#"SELECT
        manga.id AS id,
        manga.source AS source,
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
         LEFT JOIN favorite f on manga.id = f.manga_id AND f.user_id = (SELECT id FROM "user" WHERE username = ?1)
         WHERE manga.id IN RARRAY(?2)
         ORDER BY CASE manga.id
         {}"#, order
            ).as_str())?;
        let manga_ids: Vec<Value> = manga_ids.into_iter().map(Value::from).collect();
        let mangas = stmt
            .query_map(params![username, Rc::new(manga_ids)], |row| {
                let author = row
                    .get::<_, String>(3)?
                    .split(",")
                    .map(|a| a.to_string())
                    .collect();
                Ok(Manga {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    title: row.get(2)?,
                    author,
                    status: row.get(4)?,
                    description: row.get(5)?,
                    path: row.get(6)?,
                    thumbnail_url: row.get(7)?,
                    last_read: None,
                    last_page: None,
                    is_favorite: row.get(8)?,
                    genre: vec![],
                })
            })?
            .filter_map(|m| m.ok())
            .collect();

        Ok(GetMangasResponse {
            mangas,
            status: "OK".to_string(),
        })
    }

    pub fn get_manga_detail(
        &self,
        manga_id: i32,
        username: String,
    ) -> Result<GetMangaResponse, rusqlite::Error> {
        let db = self.connect_db();
        let manga = db.query_row(
            r#"SELECT
            manga.id,
            manga.source,
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
            LEFT JOIN favorite f on manga.id = f.manga_id AND f.user_id = (SELECT id FROM "user" WHERE username = ?1)
            LEFT JOIN (
                SELECT chapter.manga_id, history.chapter_id, history.last_page, MAX(history.updated) FROM chapter
                JOIN manga ON chapter.manga_id = manga.id
                JOIN history ON history.chapter_id = chapter.id AND history.user_id = (SELECT id FROM "user" WHERE username = ?1)
                GROUP BY chapter.manga_id, history.chapter_id, history.last_page
                ) h ON h.manga_id = manga.id
            WHERE manga.id = ?2"#,
            params![username, manga_id],
            |row| {
                let author = row
                    .get::<_, String>(3)?
                    .split(",")
                    .map(|a| a.to_string())
                    .collect();
                Ok(Manga {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    title: row.get(2)?,
                    author,
                    status: row.get(4)?,
                    description: row.get(5)?,
                    path: row.get(6)?,
                    thumbnail_url: row.get(7)?,
                    last_read: row.get(8)?,
                    last_page: row.get(9)?,
                    is_favorite: row.get(10)?,
                    genre: vec![],
                })
            })?;

        Ok(GetMangaResponse {
            manga,
            status: "OK".to_string(),
        })
    }

    pub fn get_chapters(
        &self,
        manga_id: i32,
        username: String,
    ) -> Result<GetChaptersResponse, rusqlite::Error> {
        let db = self.connect_db();
        let mut stmt = db.prepare(
            r#"SELECT
                chapter.id,
                chapter.source,
                chapter.manga_id,
                chapter.volume,
                chapter.number AS no, 
                COALESCE(chapter.title, '') AS title, 
                chapter.path AS url,
                COALESCE(history.last_page, 0) AS read,
                chapter.uploaded AS uploaded
            FROM chapter
            LEFT JOIN 
                history ON 
                    chapter.id = history.chapter_id AND 
                    history.user_id = (SELECT id FROM "user" WHERE username = ?1)
            WHERE 
                chapter.user_id = (SELECT id FROM "user" WHERE username = ?1) AND
                chapter.manga_id = ?2
            ORDER BY
                CAST((CASE
                    WHEN chapter.number = '' IS TRUE THEN '0'
                    ELSE chapter.number
                    END) AS DECIMAL) DESC,
                CAST((CASE
                    WHEN chapter.volume = '' IS TRUE THEN '0'
                    ELSE chapter.volume
                    END) AS DECIMAL) DESC"#,
        )?;
        let chapters = stmt
            .query_map(params![username, manga_id], |row| {
                Ok(Chapter {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    manga_id: row.get(2)?,
                    vol: row.get(3)?,
                    no: row.get(4)?,
                    title: row.get(5)?,
                    path: row.get(6)?,
                    read: row.get(7)?,
                    uploaded: row.get(8)?,
                })
            })?
            .filter_map(|c| c.ok())
            .collect::<Vec<Chapter>>();
        if !chapters.is_empty() {
            Ok(GetChaptersResponse {
                chapters,
                status: "OK".to_string(),
            })
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }

    pub fn get_pages(&self, chapter_id: i32) -> Result<GetPagesResponse, rusqlite::Error> {
        let db = self.connect_db();
        let manga_id = db.query_row(
            r#"SELECT manga.id
            FROM chapter
            JOIN manga ON manga.id = chapter.manga_id
            WHERE chapter.id = ?1"#,
            params![chapter_id],
            |row| row.get(0),
        )?;

        let mut stmt = db.prepare(
            r#"SELECT
            '/api/page/' || page.id AS url
            FROM page
            WHERE page.chapter_id = ?1
            ORDER BY page.rank"#,
        )?;
        let pages: Vec<String> = stmt
            .query_map(params![chapter_id], |row| row.get(0))?
            .filter_map(|p| p.ok())
            .collect();

        if !pages.is_empty() {
            Ok(GetPagesResponse {
                manga_id,
                pages,
                status: "OK".to_string(),
            })
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }

    pub fn get_image_from_page_id(
        &self,
        page_id: i32,
    ) -> Result<(String, String), rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            r#"SELECT source, url FROM page WHERE page.id = ?1"#,
            params![page_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok(ret)
    }

    pub fn insert_mangas(&self, source: &String, mangas: Vec<Manga>) -> Result<Vec<i32>> {
        let mut ids = vec![];
        let mut db = self.connect_db();
        let tx = db.transaction()?;
        for m in mangas {
            let last_id = match tx.query_row(
                "SELECT id FROM manga WHERE path = ?1",
                params![m.path.clone()],
                |row| row.get(0),
            ) {
                Ok(id) => id,
                Err(_) => {
                    tx.execute(
                        "INSERT INTO manga(source, title, author, status, path, thumbnail_url)
                            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            source,
                            m.title,
                            m.author.join(","),
                            m.status,
                            m.path,
                            m.thumbnail_url
                        ],
                    )?;
                    tx.last_insert_rowid()
                }
            };

            ids.push(last_id as i32);
        }
        tx.commit()?;

        Ok(ids)
    }

    pub fn insert_chapters(
        &self,
        username: String,
        manga_id: i32,
        chapters: Vec<Chapter>,
    ) -> Result<(), rusqlite::Error> {
        let mut db = self.connect_db();
        let tx = db.transaction()?;
        for c in chapters {
            tx.execute(
                r#"INSERT INTO chapter(user_id, source, manga_id, volume, number, title, path, uploaded)
                    VALUES(
                    (SELECT id FROM "user" WHERE username = ?1),
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6,
                    ?7,
                    ?8) ON CONFLICT DO NOTHING"#,
                params![username, c.source, manga_id, c.vol, c.no, c.title, c.path, c.uploaded],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn insert_pages(
        &self,
        source: String,
        chapter_id: i32,
        pages: Vec<String>,
    ) -> Result<(), rusqlite::Error> {
        let mut db = self.connect_db();
        let tx = db.transaction()?;
        for i in 0..pages.len() {
            tx.execute(
                "INSERT INTO page(source, chapter_id, rank, url)
                    VALUES(
                    ?1,
                    ?2,
                    ?3,
                    ?4) ON CONFLICT DO NOTHING",
                params![source.clone(), chapter_id, (i as i32), pages[i]],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn update_manga_info(&self, manga_id: i32, manga: Manga) -> Result<(), rusqlite::Error> {
        let db = self.connect_db();
        let a = if manga.author.is_empty() {
            None
        } else {
            Some(manga.author.join(","))
        };
        db.execute(
            "UPDATE manga SET author = COALESCE(?1, author), status = COALESCE(?2, status), description = COALESCE(?3, description) WHERE manga.id = ?4",
            params![
                a,
                manga.status,
                manga.description,
                manga_id
            ],
        )?;
        Ok(())
    }

    pub fn delete_pages(&self, chapter_id: i32) -> Result<()> {
        let db = self.connect_db();
        db.execute("DELETE FROM page WHERE chapter_id = ?", params![chapter_id])?;
        Ok(())
    }
}
