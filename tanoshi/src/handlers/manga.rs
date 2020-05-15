use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
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
    let sources = sqlx::query!("SELECT name FROM source").fetch_all(&db).await;

    let sources = sources
        .unwrap()
        .iter()
        .map(|source| source.name.clone())
        .collect::<Vec<String>>();

    Ok(warp::reply::json(&json!(
        {
            "sources": sources,
            "status": "success"
        }
    )))
}

pub async fn list_mangas(
    source: String,
    claim: Claims,
    param: Params,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    if let Ok(url) = repository::get_source_url(source.clone(), db.clone()).await {
        let mangas = match source.clone().as_str() {
            "mangasee" => Mangasee::get_mangas(&url, param, vec![]),
            "mangadex" => {
                let ret = sqlx::query!(
                    r#"SELECT mangadex_cookies FROM "user" WHERE username = $1"#,
                    claim.sub
                )
                .fetch_one(&db)
                .await;
                let ret = ret.unwrap();
                Mangadex::get_mangas(&url, param, ret.mangadex_cookies.unwrap())
            }
            &_ => return Err(warp::reject()),
        };

        match repository::insert_mangas(source.clone(), mangas.mangas.clone(), db.clone()).await {
            Ok(_) => {},
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        }

        return Ok(warp::reply::json(&mangas));
    }
    Err(warp::reject())
}

pub async fn get_manga_info(
    source: String,
    title: String,
    claim: Claims,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let title = decode_title(title);
    if let Ok(manga) =
        repository::get_manga_detail(source.clone(), title.clone(), claim.sub.clone(), db.clone())
            .await
    {
        return Ok(warp::reply::json(&manga));
    } else if let Ok(url) =
        repository::get_manga_url(source.clone(), title.clone(), db.clone()).await
    {
        let manga = match source.as_str() {
            "mangasee" => Mangasee::get_manga_info(&url),
            "mangadex" => Mangadex::get_manga_info(&url),
            _ => return Err(warp::reject()),
        };

        sqlx::query!(
            "UPDATE manga SET author = $1, status = $2, description = $3
                WHERE manga.source_id = (
                SELECT source.id FROM source
                WHERE source.name = $4)
                AND manga.title = $5",
            manga.manga.author,
            manga.manga.status,
            manga.manga.description,
            source,
            title,
        )
        .execute(&db)
        .await;

        return Ok(warp::reply::json(&manga));
    }
    Err(warp::reject())
}

pub async fn get_chapters(
    source: String,
    title: String,
    claim: Claims,
    param: GetParams,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let title = decode_title(title);
    if !param.refresh.unwrap_or(false) {
        match repository::get_chapters(source.clone(), title.clone(), claim.sub.clone(), db.clone()).await {
            Ok(chapter) => return Ok(warp::reply::json(&chapter)),
            Err(_e) => {}
        };
    }

    if let Ok(url) = repository::get_manga_url(source.clone(), title.clone(), db.clone()).await {
        let chapter = match source.as_str() {
            "mangasee" => Mangasee::get_chapters(&url),
            "mangadex" => Mangadex::get_chapters(&url),
            _ => return Err(warp::reject()),
        };

        match repository::insert_chapters(source.clone(), title.clone(), chapter.chapters.clone(), db.clone()).await {
            Ok(_) => {},
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        }

        match repository::get_chapters(source.clone(), title.clone(), claim.sub, db.clone()).await {
            Ok(chapter) => return Ok(warp::reply::json(&chapter)),
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        };
    }
    Err(warp::reject())
}

pub async fn get_pages(
    source: String,
    title: String,
    chapter: String,
    _param: GetParams,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let title = decode_title(title);
    match repository::get_pages(source.clone(), title.clone(), chapter.clone(), db.clone()).await {
        Ok(pages) => return Ok(warp::reply::json(&pages)),
        Err(_) => {}
    };

    if let Ok(url) =
        repository::get_chapter_url(source.clone(), title.clone(), chapter.clone(), db.clone())
            .await
    {
        let pages = match source.as_str() {
            "mangasee" => Mangasee::get_pages(&url),
            "mangadex" => Mangadex::get_pages(&url),
            _ => return Err(warp::reject()),
        };

        match repository::insert_pages(source.clone(), title.clone(), chapter.clone(), pages.pages.clone(), db.clone()).await {
            Ok(_) => {},
            Err(e) => return Err(warp::reject::custom(TransactionReject{message: e.to_string()})),
        }

        match repository::get_pages(source.clone(), title.clone(), chapter.clone(), db.clone()).await {
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
    reader.read_to_end(&mut bytes);

    let resp = Response::builder()
    .header("Content-Type", content_type)
    .header("Content-Length", len)
    .body(bytes)
    .unwrap();

    Ok(resp)
}

pub async fn login(
    claim: Claims,
    login: MangadexLogin,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    if let Ok(url) = repository::get_source_url("mangadex".to_owned(), db.clone()).await {
        match Mangadex::login(&url, login) {
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
    Err(warp::reject())
}

fn encode_title(title: String) -> String {
    base64::encode_config(&title, base64::URL_SAFE_NO_PAD)
}

fn decode_title(encoded: String) -> String {
    String::from_utf8(base64::decode_config(encoded, base64::URL_SAFE_NO_PAD).unwrap()).unwrap()
}
