pub mod mangasee {
    use crate::scraper::{mangasee::Mangasee, Chapter, Manga, Params, Scraping};
    use std::collections::BTreeMap;
    use std::convert::Infallible;

    pub async fn list_mangas(
        param: Params,
        mangasee: Mangasee,
    ) -> Result<impl warp::Reply, Infallible> {
        let mangas = mangasee.get_mangas(param);
        Ok(warp::reply::json(&mangas))
    }

    pub async fn get_manga_info(
        title: String,
        mangasee: Mangasee,
    ) -> Result<impl warp::Reply, Infallible> {
        let manga = mangasee.get_manga_info(&Manga {
            title: "".to_string(),
            author: "".to_string(),
            genre: vec![],
            status: "".to_string(),
            description: "".to_string(),
            url: format!("/manga/{}", title),
            thumbnail_url: "".to_string(),
            chapter: BTreeMap::new(),
        });
        Ok(warp::reply::json(&manga))
    }

    pub async fn get_chapter(
        title: String,
        chapter: String,
        mangasee: Mangasee,
    ) -> Result<impl warp::Reply, Infallible> {
        let mut chapter = Chapter {
            prev_chapter: "".to_string(),
            chapter: chapter.to_owned(),
            next_chapter: "".to_string(),
            url: format!("/read-online/{}-chapter-{}.html", title, chapter),
            pages: vec![],
        };

        mangasee.get_chapter(&mut chapter);
        Ok(warp::reply::json(&chapter))
    }
}
