use rusqlite::types::Value;
use rusqlite::vtab::array;
use rusqlite::{params, Connection, Row};
use tanoshi_lib::manga::{
    Chapter, GetChaptersResponse, GetMangaResponse, GetMangasResponse, GetPagesResponse, Image,
    Manga, Source,
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

    fn source_from_row(row: &Row) -> Result<Source, rusqlite::Error> {
        Ok(Source {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            version: row.get(3)?,
        })
    }

    pub async fn get_sources(&self) -> Result<Vec<Source>, rusqlite::Error> {
        let db = self.connect_db();
        let mut stmt = db.prepare("SELECT id, name, url, version FROM source")?;
        let sources = stmt
            .query_map(params![], Repository::source_from_row)?
            .filter_map(|s| s.ok())
            .collect();
        Ok(sources)
    }

    pub async fn get_source(&self, source_id: i32) -> Result<Source, rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            r#"SELECT id, name, url, version FROM source WHERE id = ?1"#,
            params![source_id],
            Repository::source_from_row,
        )?;

        Ok(ret)
    }

    pub async fn get_source_from_manga_id(&self, manga_id: i32) -> Result<Source, rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            r#"SELECT source.id, source.name, source.url, source.version
            FROM manga
            JOIN source ON source.id = manga.source_id
            WHERE manga.id = ?1"#,
            params![manga_id],
            Repository::source_from_row,
        )?;

        Ok(ret)
    }

    pub async fn get_image_from_page_id(&self, page_id: i32) -> Result<Image, rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            r#"SELECT
            s.id AS source_id,
            s.name AS source_name,
            s.id || '/' || m.id || '/' || c.id AS path,
            page.rank || '.' || replace(page.url, rtrim(page.url, replace(page.url, '.', '')), '') AS file_name,
            page.url AS url
            FROM page
            JOIN chapter c on page.chapter_id = c.id
            JOIN manga m on c.manga_id = m.id
            JOIN source s on m.source_id = s.id
            WHERE page.id = ?1"#,
            params![page_id],
            |row| {
                Ok(Image {
                    source_id: row.get(0)?,
                    source_name: row.get(1)?,
                    path: row.get(2)?,
                    file_name: row.get(3)?,
                    url: row.get(4)?,
                })
            },
        )?;

        Ok(ret)
    }

    pub async fn get_source_from_chapter_id(
        &self,
        chapter_id: i32,
    ) -> Result<Source, rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            r#"SELECT source.id, source.name, source.url, source.version
            FROM chapter
            JOIN manga ON manga.id = chapter.manga_id
            JOIN source ON source.id = manga.source_id
            WHERE chapter.id = ?1"#,
            params![chapter_id],
            Repository::source_from_row,
        )?;

        Ok(ret)
    }

    pub async fn get_manga_url(&self, manga_id: i32) -> Result<String, rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            r#"SELECT source.url || manga.path AS url FROM manga
                JOIN source ON source.id = manga.source_id
                WHERE manga.id = ?1"#,
            params![manga_id],
            |row| row.get(0),
        )?;

        Ok(ret)
    }

    pub async fn get_chapter_url(&self, chapter_id: i32) -> Result<String, rusqlite::Error> {
        let db = self.connect_db();
        let ret = db.query_row(
            "SELECT source.url || chapter.path AS url FROM chapter
                JOIN manga ON manga.id = chapter.manga_id
                JOIN source ON source.id = manga.source_id
                WHERE chapter.id = ?1",
            params![chapter_id],
            |row| row.get(0),
        )?;

        Ok(ret)
    }

    pub async fn get_mangas(
        &self,
        username: String,
        manga_ids: Vec<i32>,
    ) -> Result<GetMangasResponse, rusqlite::Error> {
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
                Ok(Manga {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    author: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                    path: row.get(5)?,
                    thumbnail_url: row.get(6)?,
                    last_read: None,
                    last_page: None,
                    is_favorite: row.get(7)?,
                })
            })?
            .filter_map(|m| m.ok())
            .collect();

        Ok(GetMangasResponse { mangas })
    }

    pub async fn get_manga_detail(
        &self,
        manga_id: i32,
        username: String,
    ) -> Result<GetMangaResponse, rusqlite::Error> {
        let db = self.connect_db();
        let manga = db.query_row(
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
            LEFT JOIN favorite f on manga.id = f.manga_id AND f.user_id = (SELECT id FROM "user" WHERE username = ?1)
            LEFT JOIN (
                SELECT chapter.manga_id, history.chapter_id, history.last_page, MAX(history.updated) FROM chapter
                JOIN manga ON chapter.manga_id = manga.id
                JOIN history ON history.chapter_id = chapter.id AND history.user_id = (SELECT id FROM "user" WHERE username = ?1)
                GROUP BY chapter.manga_id, history.chapter_id, history.last_page
                ) h ON h.manga_id = manga.id
            WHERE manga.id = ?2"#,
            params![username, manga_id],
            |row| Ok(Manga {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                status: row.get(3)?,
                description: row.get(4)?,
                path: row.get(5)?,
                thumbnail_url: row.get(6)?,
                last_read: row.get(7)?,
                last_page: row.get(8)?,
                is_favorite: row.get(9)?,
            }))?;

        Ok(GetMangaResponse { manga })
    }

    pub async fn get_chapters(
        &self,
        manga_id: i32,
        username: String,
    ) -> Result<GetChaptersResponse, rusqlite::Error> {
        let db = self.connect_db();
        let mut stmt = db.prepare(
            r#"SELECT
            chapter.id,
            chapter.manga_id,
            chapter.number AS no, COALESCE(chapter.title, '') AS title, chapter.path AS url,
            COALESCE(history.last_page, 0) AS read,
            chapter.uploaded AS uploaded
            FROM chapter
            LEFT JOIN history ON chapter.id = history.chapter_id
            AND history.user_id = (SELECT id FROM "user" WHERE username = ?1)
            WHERE chapter.manga_id = ?2
            ORDER BY CAST((CASE
                WHEN chapter.number = '' IS TRUE THEN '0'
                ELSE chapter.number
                END) AS DECIMAL) DESC"#,
        )?;
        let chapters = stmt
            .query_map(params![username, manga_id], |row| {
                Ok(Chapter {
                    id: row.get(0)?,
                    manga_id: row.get(1)?,
                    no: row.get(2)?,
                    title: row.get(3)?,
                    url: row.get(4)?,
                    read: row.get(5)?,
                    uploaded: row.get(6)?,
                })
            })?
            .filter_map(|c| c.ok())
            .collect::<Vec<Chapter>>();
        if !chapters.is_empty() {
            Ok(GetChaptersResponse { chapters })
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }

    pub async fn get_pages(&self, chapter_id: i32) -> Result<GetPagesResponse, rusqlite::Error> {
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
            Ok(GetPagesResponse { manga_id, pages })
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }

    pub async fn insert_sources(&self, sources: Vec<Source>) -> Result<(), rusqlite::Error> {
        let mut db = self.connect_db();
        let tx = db.transaction()?;
        for source in sources {
            tx.execute(
                r"INSERT INTO source(name, url, version) VALUES (?1, ?2, ?3) ON CONFLICT(name) DO UPDATE SET url=EXCLUDED.url",
                params![source.name,
                source.url,
                source.version],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub async fn insert_mangas(&self, source_id: i32, mangas: Vec<Manga>) -> Result<Vec<i32>> {
        let mut ids = vec![];
        let mut db = self.connect_db();
        let tx = db.transaction()?;
        for m in mangas {
            let last_id = match tx.query_row(
                "SELECT id FROM manga WHERE source_id = ?1 AND path = ?2",
                params![source_id, m.path.clone()],
                |row| row.get(0),
            ) {
                Ok(id) => id,
                Err(_) => {
                    tx.execute(
                        "INSERT INTO manga(source_id, title, path, thumbnail_url)
                            VALUES (?1, ?2, ?3, ?4)
                            ON CONFLICT(source_id, path) DO UPDATE SET thumbnail_url=EXCLUDED.thumbnail_url",
                        params![source_id,m.title,m.path,m.thumbnail_url])?;
                    tx.last_insert_rowid()
                }
            };

            ids.push(last_id as i32);
        }
        tx.commit()?;

        Ok(ids)
    }

    pub async fn insert_chapters(
        &self,
        manga_id: i32,
        chapters: Vec<Chapter>,
    ) -> Result<(), rusqlite::Error> {
        let mut db = self.connect_db();
        let tx = db.transaction()?;
        for c in chapters {
            tx.execute(
                "INSERT INTO chapter(manga_id, number, title, path, uploaded)
                    VALUES(
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5) ON CONFLICT DO NOTHING",
                params![manga_id, c.no, c.title, c.url, c.uploaded],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub async fn insert_pages(
        &self,
        chapter_id: i32,
        pages: Vec<String>,
    ) -> Result<(), rusqlite::Error> {
        let mut db = self.connect_db();
        let tx = db.transaction()?;
        for i in 0..pages.len() {
            tx.execute(
                "INSERT INTO page(chapter_id, rank, url)
                    VALUES(
                    ?1,
                    ?2,
                    ?3) ON CONFLICT DO NOTHING",
                params![chapter_id, (i as i32), pages[i]],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub async fn update_manga_info(
        &self,
        manga_id: i32,
        manga: Manga,
    ) -> Result<(), rusqlite::Error> {
        let db = self.connect_db();
        db.execute(
            "UPDATE manga SET author = $1, status = $2, description = $3 WHERE manga.id = $4",
            params![manga.author, manga.status, manga.description, manga_id],
        )?;
        Ok(())
    }
}
