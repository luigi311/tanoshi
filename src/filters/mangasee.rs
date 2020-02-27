pub mod mangasee {
    use crate::handlers::mangasee::mangasee;
    use crate::scraper::{mangasee::Mangasee, Params};
    use warp::Filter;

    pub fn mangasee(
        mangasee: Mangasee,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        list_mangas(mangasee)
            .or(get_manga_info(mangasee))
            .or(get_chapters(mangasee))
            .or(get_pages(mangasee))
    }

    pub fn list_mangas(
        mangasee: Mangasee,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangasee")
            .and(warp::get())
            .and(warp::query::<Params>())
            .and(with_mangasee(mangasee))
            .and_then(mangasee::list_mangas)
    }

    pub fn get_manga_info(
        mangasee: Mangasee,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangasee" / "manga" / String)
            .and(warp::get())
            .and(with_mangasee(mangasee))
            .and_then(mangasee::get_manga_info)
    }

    pub fn get_chapters(
        mangasee: Mangasee,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangasee" / "manga" / String / "chapter")
            .and(warp::get())
            .and(with_mangasee(mangasee))
            .and_then(mangasee::get_chapters)
    }

    pub fn get_pages(
        mangasee: Mangasee,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangasee" / "manga" / String / "chapter" / String)
            .and(warp::get())
            .and(with_mangasee(mangasee))
            .and_then(mangasee::get_pages)
    }

    fn with_mangasee(
        mangasee: Mangasee,
    ) -> impl Filter<Extract = (Mangasee,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || mangasee.clone())
    }
}
