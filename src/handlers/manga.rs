pub mod manga {
    use crate::scraper::{mangasee::Mangasee, GetParams, Params, Scraping};
    use sled::{Batch, Tree};
    use warp::Rejection;

    pub async fn list_mangas(
        source: String,
        param: Params,
        db: Tree,
    ) -> Result<impl warp::Reply, Rejection> {
        if let Ok(url) = get_source_url(source.clone(), db.clone()) {
            let mangas = Mangasee::get_mangas(&url, param);
            db.insert(
                &url,
                serde_json::to_vec(&mangas).expect("failed to serialize data"),
            )
            .unwrap();

            let mut batch = Batch::default();
            for m in mangas.clone().mangas {
                let key = format!(
                    "{}:{}:path",
                    source.clone(),
                    base64::encode_config(&m.title, base64::URL_SAFE_NO_PAD)
                );
                batch.insert(key.as_str(), m.path.as_str());
                let key = format!(
                    "{}:{}:thumbnail",
                    source.clone(),
                    base64::encode_config(&m.title, base64::URL_SAFE_NO_PAD)
                );
                batch.insert(key.as_str(), m.thumbnail_url.as_str());
            }
            db.apply_batch(batch).expect("failed to insert mangas");

            return Ok(warp::reply::json(&mangas));
        }
        Err(warp::reject())
    }

    pub async fn get_manga_info(
        source: String,
        title: String,
        db: Tree,
    ) -> Result<impl warp::Reply, Rejection> {
        if let Ok(url) = get_manga_url(source.clone(), title.clone(), db.clone()) {
            let manga = match db.get(&url).unwrap() {
                Some(bytes) => serde_json::from_slice(&bytes).expect("failed to deserialize data"),
                None => {
                    let manga = Mangasee::get_manga_info(&url);
                    db.insert(
                        &url,
                        serde_json::to_vec(&manga).expect("failed to serialize data"),
                    )
                    .unwrap();
                    manga
                }
            };
            return Ok(warp::reply::json(&manga));
        }
        Err(warp::reject())
    }

    pub async fn get_chapters(
        source: String,
        title: String,
        param: GetParams,
        db: Tree,
    ) -> Result<impl warp::Reply, Rejection> {
        if let Ok(url) = get_manga_url(source.clone(), title.clone(), db.clone()) {
            let key = format!("{}:chapters", &url);
            let chapter = match db.get(&key).unwrap() {
                Some(bytes) => serde_json::from_slice(&bytes).expect("failed to deserialize data"),
                None => {
                    let chapter = Mangasee::get_chapters(&url);
                    db.insert(
                        &key,
                        serde_json::to_vec(&chapter).expect("failed to serialize data"),
                    )
                    .unwrap();

                    let mut batch = Batch::default();
                    for c in chapter.clone().chapters {
                        let key = format!("{}:{}:{}", source.clone(), title.clone(), c.no.clone());
                        batch.insert(key.as_str(), c.url.as_str());
                    }
                    db.apply_batch(batch).expect("failed to insert mangas");
                    chapter
                }
            };
            return Ok(warp::reply::json(&chapter));
        }
        Err(warp::reject())
    }

    pub async fn get_pages(
        source: String,
        title: String,
        chapter: String,
        param: GetParams,
        db: Tree,
    ) -> Result<impl warp::Reply, Rejection> {
        if let Ok(url) = get_chapter_url(source.clone(), title.clone(), chapter.clone(), db.clone())
        {
            let pages = match db.get(&url).unwrap() {
                Some(bytes) => serde_json::from_slice(&bytes).expect("failed to deserialize data"),
                None => {
                    let pages = Mangasee::get_pages(&url);
                    db.insert(
                        &url,
                        serde_json::to_vec(&pages).expect("failed to serialize data"),
                    )
                    .unwrap();
                    pages
                }
            };
            return Ok(warp::reply::json(&pages));
        }
        Err(warp::reject())
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

        let key = format!("{}:{}:path", source, title);
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
