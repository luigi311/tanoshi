use std::sync::{Arc, Mutex};

use sqlx::postgres::PgPool;
use warp::{http::Response, Rejection};

use serde_json::json;
use std::io::Read;
use ureq;

use crate::auth::Claims;
use crate::scraper::{mangadex::Mangadex, mangasee::Mangasee, repository, Scraping};
use tanoshi::manga::{GetParams, ImageProxyParam, Params};
use tanoshi::mangadex::MangadexLogin;

use crate::handlers::TransactionReject;

pub async fn list_sources(db: PgPool) -> Result<impl warp::Reply, Rejection> {
    match repository::get_sources(db).await {
        Ok(sources) => Ok(warp::reply::json(&json!(
            {
                "sources": sources,
                "status": "success"
            }
        ))),
        Err(e) => Err(warp::reject::custom(TransactionReject{message: e.to_string()}))
    }
}

pub async fn list_mangas(
    source_id: i32,
    claim: Claims,
    param: Params,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    if let Ok(source) = repository::get_source(source_id, db.clone()).await {
        let mangas = match source.name.as_str() {
            "mangasee" => Mangasee::get_mangas(&source.url, param, vec![]),
            "mangadex" => {
                let ret = sqlx::query!(
                    r#"SELECT mangadex_cookies FROM "user" WHERE username = $1"#,
                    claim.sub.clone()
                )
                .fetch_one(&db)
                .await;
                let ret = ret.unwrap();
                Mangadex::get_mangas(&source.url, param, ret.mangadex_cookies.unwrap())
            }
            &_ => return Err(warp::reject()),
        };

        let manga_ids = match repository::insert_mangas(source_id, mangas.mangas.clone(), db.clone()).await {
            Ok(ids) => ids,
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        };
        match repository::get_mangas(claim.sub, manga_ids, db).await {
            Ok(mangas) => return Ok(warp::reply::json(&mangas)),
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        }
    }
    Err(warp::reject())
}

pub async fn get_manga_info(
    manga_id: i32,
    claim: Claims,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    if let Ok(manga) =
        repository::get_manga_detail(manga_id, claim.sub.clone(), db.clone())
            .await
    {
        return Ok(warp::reply::json(&manga));
    } else if let Ok(url) = repository::get_manga_url(manga_id, db.clone()).await
    {
        let manga = if url.contains("mangasee") {
            Mangasee::get_manga_info(&url)
        } else if url.contains("mangadex") {
            Mangadex::get_manga_info(&url)
        } else {
            return Err(warp::reject());
        };

        match repository::update_manga_info(manga_id, manga.manga, db.clone()).await {
            Ok(_) => {},
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        }
        match repository::get_manga_detail(manga_id, claim.sub, db).await {
            Ok(res) => return Ok(warp::reply::json(&res)),
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        }
    }
    Err(warp::reject())
}

pub async fn get_chapters(
    manga_id: i32,
    claim: Claims,
    param: GetParams,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    if !param.refresh.unwrap_or(false) {
        match repository::get_chapters(manga_id, claim.sub.clone(), db.clone()).await {
            Ok(chapter) => return Ok(warp::reply::json(&chapter)),
            Err(_e) => {}
        };
    }

    if let Ok(url) = repository::get_manga_url(manga_id, db.clone()).await {
        let chapter = if url.contains("mangasee") {
            Mangasee::get_chapters(&url)
        } else if url.contains("mangadex") {
            Mangadex::get_chapters(&url)
        } else {
            return Err(warp::reject());
        };

        match repository::insert_chapters(manga_id, chapter.chapters.clone(), db.clone()).await {
            Ok(_) => {},
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        }

        match repository::get_chapters(manga_id, claim.sub, db.clone()).await {
            Ok(chapter) => return Ok(warp::reply::json(&chapter)),
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        };
    }
    Err(warp::reject())
}

pub async fn get_pages(
    chapter_id: i32,
    _param: GetParams,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    match repository::get_pages(chapter_id, db.clone()).await {
        Ok(pages) => return Ok(warp::reply::json(&pages)),
        Err(_) => {}
    };

    if let Ok(url) = repository::get_chapter_url(chapter_id, db.clone()).await
    {
        let pages = if url.contains("mangasee") {
            Mangasee::get_pages(&url)
        } else if url.contains("mangadex") {
            Mangadex::get_pages(&url)
        } else {
            return Err(warp::reject());
        };

        match repository::insert_pages(chapter_id, pages.pages.clone(), db.clone()).await {
            Ok(_) => {},
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        }

        match repository::get_pages(chapter_id, db.clone()).await {
            Ok(pages) => return Ok(warp::reply::json(&pages)),
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        };
    }
    Err(warp::reject())
}

pub async fn proxy_image(param: ImageProxyParam) -> Result<impl warp::Reply, Rejection> {
    let resp = ureq::get(&param.url).call();
    let len = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap();
    let content_type = resp.content_type().to_owned();

    let mut reader = resp.into_reader();
    let mut bytes = vec![];
    reader.read_to_end(&mut bytes).expect("error write image");

    let resp = Response::builder()
    .header("Content-Type", content_type)
    .header("Content-Length", bytes.len())
    .body(bytes)
    .unwrap();

    Ok(resp)
}

pub async fn login(
    claim: Claims,
    login: MangadexLogin,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    match Mangadex::login(&"https://mangadex.org".to_owned(), login) {
        Ok(cookies) => {
            sqlx::query!(
                r#"
                UPDATE "user"
                SET mangadex_cookies = $1
                WHERE username = $2"#,
                cookies.as_slice(),
                claim.sub,
            )
            .execute(&db)
            .await;
            return Ok(warp::reply());
        }
        Err(e) => return Err(warp::reject::custom(TransactionReject{message: e})),
    }
}