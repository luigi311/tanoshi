pub mod mangasee {
    use crate::scraper::{mangasee::Mangasee, Chapter, Manga, Params, Scraping};
    use std::convert::Infallible;
    use warp::http::StatusCode;

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
        let mut url = String::from("/manga/");
        url.push_str(&title);

        let manga = mangasee.get_manga_info(&Manga {
            title: "".to_string(),
            author: "".to_string(),
            genre: vec![],
            status: "".to_string(),
            description: "".to_string(),
            url: url.to_owned(),
            thumbnail_url: "".to_string(),
            chapter: Default::default(),
        });
        Ok(warp::reply::json(&manga))
    }

    pub async fn get_chapter(
        title: String,
        chapter: String,
        mangasee: Mangasee,
    ) -> Result<impl warp::Reply, Infallible> {
        let mut url = String::from("/read-online/");
        url.push_str(&title);
        url.push_str("-chapter-");
        url.push_str(&chapter);
        url.push_str(".html");
        println!("{:?}", url);
        let mut chapter = Chapter {
            url: url.to_owned(),
            pages: vec![],
        };
        mangasee.get_chapter(&mut chapter);
        Ok(warp::reply::json(&chapter))
    }
}
