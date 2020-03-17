pub mod mangasee {
    use crate::auth::auth::Auth;
    use crate::auth::Claims;
    use crate::filters::settings::settings::auth_handler;
    use crate::handlers::mangasee::mangasee;
    use crate::scraper::{mangasee::Mangasee, GetParams, Params};
    use sled::Db;
    use warp::Filter;

    pub fn mangasee(
        mangasee: Mangasee,
        auth: Auth,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        list_mangas(mangasee.clone())
            .or(get_manga_info(mangasee.clone(), db.clone()))
            .or(get_chapters(mangasee.clone(), db.clone()))
            .or(get_pages(mangasee, db.clone()))
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
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangasee" / "manga" / String)
            .and(warp::get())
            .and(with_mangasee(mangasee))
            .and(with_db(db))
            .and_then(mangasee::get_manga_info)
    }

    pub fn get_chapters(
        mangasee: Mangasee,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangasee" / "manga" / String / "chapter")
            .and(warp::get())
            .and(warp::query::<GetParams>())
            .and(with_mangasee(mangasee))
            .and(with_db(db))
            .and_then(mangasee::get_chapters)
    }

    pub fn get_pages(
        mangasee: Mangasee,
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "source" / "mangasee" / "manga" / String / "chapter" / String)
            .and(warp::get())
            .and(warp::query::<GetParams>())
            .and(with_mangasee(mangasee))
            .and(with_db(db))
            .and_then(mangasee::get_pages)
    }

    fn with_db(db: Db) -> impl Filter<Extract = (Db,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || db.clone())
    }

    fn with_mangasee(
        mangasee: Mangasee,
    ) -> impl Filter<Extract = (Mangasee,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || mangasee.clone())
    }
}
