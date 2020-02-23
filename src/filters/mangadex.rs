pub mod mangadex {
    use crate::handlers::mangadex::mangadex;
    use crate::scraper::{mangadex::Mangadex, Params};
    use warp::Filter;

    pub fn mangadex(
        mangadex: Mangadex,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        list_mangas(mangadex)
            .or(get_manga_info(mangadex))
            .or(get_chapter(mangadex))
    }

    pub fn list_mangas(
        mangadex: Mangadex,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangadex")
            .and(warp::get())
            .and(warp::query::<Params>())
            .and(with_mangasee(mangadex))
            .and_then(mangadex::list_mangas)
    }

    pub fn get_manga_info(
        mangadex: Mangadex,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangadex" / "manga" / String)
            .and(warp::get())
            .and(with_mangasee(mangadex))
            .and_then(mangadex::get_manga_info)
    }

    pub fn get_chapter(
        mangadex: Mangadex,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangadex" / "manga" / String / "chapter" / String)
            .and(warp::get())
            .and(with_mangasee(mangadex))
            .and_then(mangadex::get_chapter)
    }

    fn with_mangasee(
        mangadex: Mangadex,
    ) -> impl Filter<Extract = (Mangadex,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || mangadex.clone())
    }
}
