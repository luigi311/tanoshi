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
        let manga = mangasee.get_manga_info(format!("/manga/{}", title));
        Ok(warp::reply::json(&manga))
    }

    pub async fn get_chapter(
        title: String,
        chapter: String,
        mangasee: Mangasee,
    ) -> Result<impl warp::Reply, Infallible> {
        let chapter =
            mangasee.get_chapter(format!("/read-online/{}-chapter-{}.html", title, chapter));
        Ok(warp::reply::json(&chapter))
    }
}
