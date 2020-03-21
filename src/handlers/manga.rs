pub mod manga {
    use std::convert::Infallible;

    use sled::{Batch, Tree};

    use crate::auth::Claims;
    use crate::scraper::{mangasee::Mangasee, GetParams, Params, Scraping};
    use warp::Rejection;

    pub async fn list_mangas(
        source: String,
        param: Params,
        db: Tree,
    ) -> Result<impl warp::Reply, Rejection> {
        match get_source_url(source.clone(), db.clone()) {
            Ok(url) => {
                let mangas = Mangasee::get_mangas(url, param);
                let mut batch = Batch::default();
                for m in mangas.clone().mangas {
                    let key = format!("{}:{}", source.clone(), base64::encode(m.title));
                    batch.insert(key.as_str(), m.path.as_str());
                }
                db.apply_batch(batch).expect("failed to insert mangas");
                Ok(warp::reply::json(&mangas))
            }
            Err(e) => Err(warp::reject()),
        }
    }

    pub async fn get_manga_info(
        source: String,
        title: String,
        db: Tree,
    ) -> Result<impl warp::Reply, Rejection> {
        match get_manga_url(source, title, db) {
            Ok(url) => {
                let manga = Mangasee::get_manga_info(url);
                Ok(warp::reply::json(&manga))
            }
            Err(e) => Err(warp::reject()),
        }
    }

    pub async fn get_chapters(
        source: String,
        title: String,
        param: GetParams,
        db: Tree,
    ) -> Result<impl warp::Reply, Rejection> {
        match get_manga_url(source.clone(), title.clone(), db.clone()) {
            Ok(url) => {
                let chapter = Mangasee::get_chapters(url);
                let mut batch = Batch::default();
                for c in chapter.clone().chapters {
                    let key = format!("{}:{}:{}", source.clone(), title.clone(), c.no.clone());
                    batch.insert(key.as_str(), c.url.as_str());
                }
                db.apply_batch(batch).expect("failed to insert mangas");
                Ok(warp::reply::json(&chapter))
            }
            Err(e) => Err(warp::reject()),
        }
    }

    pub async fn get_pages(
        source: String,
        title: String,
        chapter: String,
        param: GetParams,
        db: Tree,
    ) -> Result<impl warp::Reply, Rejection> {
        match get_chapter_url(source, title, chapter, db) {
            Ok(url) => {
                let pages = Mangasee::get_pages(url);
                Ok(warp::reply::json(&pages))
            }
            Err(e) => Err(warp::reject()),
        }
    }

    fn get_source_url(source: String, db: Tree) -> Result<String, String> {
        match db.get(source) {
            Ok(res) => Ok(String::from_utf8(res.unwrap().to_vec()).unwrap()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn get_manga_url(source: String, title: String, db: Tree) -> Result<String, String> {
        let base_url = match get_source_url(source.clone(), db.clone()) {
            Ok(res) => res,
            Err(e) => return Err(e.to_string()),
        };

        let key = format!("{}:{}", source, title);
        println!("get key {}", key.clone());
        let path = match db.get(key) {
            Ok(res) => String::from_utf8(res.unwrap().to_vec()).unwrap(),
            Err(e) => return Err(e.to_string()),
        };

        Ok(format!("{}{}", base_url, path))
    }

    fn get_chapter_url(
        source: String,
        title: String,
        chapter: String,
        db: Tree,
    ) -> Result<String, String> {
        let base_url = match get_source_url(source.clone(), db.clone()) {
            Ok(res) => res,
            Err(e) => return Err(e.to_string()),
        };

        let path = match db.get(format!("{}:{}:{}", source, title, chapter)) {
            Ok(res) => String::from_utf8(res.unwrap().to_vec()).unwrap(),
            Err(e) => return Err(e.to_string()),
        };

        Ok(format!("{}{}", base_url, path))
    }
}
