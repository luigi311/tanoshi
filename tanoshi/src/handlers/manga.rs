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

        for m in mangas.clone().mangas {
            sqlx::query!(
                "INSERT INTO manga(
                    source_id,
                    title,
                    path,
                    thumbnail_url
                    ) VALUES (
                    (SELECT id FROM source WHERE name = $1),
                    $2,
                    $3,
                    $4) ON CONFLICT DO NOTHING",
                source,
                m.title,
                m.path,
                m.thumbnail_url,
            )
            .execute(&db)
            .await;
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
        match repository::get_chapters(source.clone(), title.clone(), claim.sub, db.clone()).await {
            Ok(chapter) => return Ok(warp::reply::json(&chapter)),
            Err(e) => {}
        };
    }

    if let Ok(url) = repository::get_manga_url(source.clone(), title.clone(), db.clone()).await {
        let chapter = match source.as_str() {
            "mangasee" => Mangasee::get_chapters(&url),
            "mangadex" => Mangadex::get_chapters(&url),
            _ => return Err(warp::reject()),
        };

        for c in chapter.clone().chapters {
            sqlx::query!(
                "INSERT INTO chapter(manga_id, number, title, path, uploaded)
                VALUES(
                (SELECT manga.id FROM manga
                JOIN source ON source.id = manga.source_id
                WHERE source.name = $1 AND title = $2 ),
                $3,
                $4,
                $5,
                $6) ON CONFLICT DO NOTHING",
                source,
                title,
                c.no,
                c.title,
                c.url,
                c.uploaded,
            )
            .execute(&db)
            .await;
        }

        let mut chapter = chapter;
        chapter.chapters.sort();
        chapter.chapters.reverse();
        return Ok(warp::reply::json(&chapter));
    }
    Err(warp::reject())
}

pub async fn get_pages(
    source: String,
    title: String,
    chapter: String,
    param: GetParams,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    let title = decode_title(title);
    if let Ok(url) =
        repository::get_chapter_url(source.clone(), title.clone(), chapter.clone(), db.clone())
            .await
    {
        let pages = match source.as_str() {
            "mangasee" => Mangasee::get_pages(&url),
            "mangadex" => Mangadex::get_pages(&url),
            _ => return Err(warp::reject()),
        };
        for i in 0..pages.pages.len() {
            sqlx::query!(
                "INSERT INTO page(chapter_id, rank, url)
                VALUES(
                (SELECT chapter.id FROM chapter
                JOIN manga ON manga.id = chapter.manga_id
                JOIN source ON source.id = manga.source_id
                WHERE source.name = $1 AND manga.title = $2 AND chapter.number = $3),
                $4,
                $5) ON CONFLICT DO NOTHING",
                source,
                title,
                chapter,
                (i as i32),
                pages.pages[i],
            )
            .execute(&db)
            .await;
        }
        return Ok(warp::reply::json(&pages));
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
    source: String,
    claim: Claims,
    login: MangadexLogin,
    db: PgPool,
) -> Result<impl warp::Reply, Rejection> {
    if let Ok(url) = repository::get_source_url(source.clone(), db.clone()).await {
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
            Err(_) => return Err(warp::reject()),
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
