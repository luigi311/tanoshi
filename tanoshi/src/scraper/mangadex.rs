use fancy_regex::Regex;
use serde_json::json;
use ureq;

use crate::scraper::Scraping;
use tanoshi::manga::{
    Chapter, GetChaptersResponse, GetMangaResponse, GetMangasResponse, GetPagesResponse, GetParams,
    Manga, Params, SortByParam, SortOrderParam
};
use chrono::{DateTime, Local};
use tanoshi::mangadex::MangadexLogin;

#[derive(Clone)]
pub struct Mangadex {
    pub url: &'static str,
}

impl Scraping for Mangadex {
    fn get_mangas(url: &String, param: Params, cookies: Vec<String>) -> GetMangasResponse {
        let mut mangas: Vec<Manga> = Vec::new();

        let mut s = match param.sort_by.unwrap() {
            SortByParam::LastUpdated => 0,
            SortByParam::Views => 8,
            SortByParam::Title => 2,
            _ => 0,
        };

        s = match param.sort_order.unwrap() {
            SortOrderParam::Asc => s,
            SortOrderParam::Desc => s + 1,
        };

        let params = vec![
            ("title".to_owned(), param.keyword.to_owned()),
            ("p".to_owned(), param.page.to_owned()),
            ("s".to_owned(), Some(s.to_string())),
        ];

        let urlencoded = serde_urlencoded::to_string(params).unwrap();

        let resp = ureq::get(format!("{}/search?{}", url.clone(), urlencoded).as_str())
        .set("Cookie", &cookies.join("; "))
        .call();

        let html = resp.into_string().unwrap();
        let document = scraper::Html::parse_document(&html);

        let selector = scraper::Selector::parse(".manga-entry").unwrap();
        for row in document.select(&selector) {
            let mut manga = Manga::default();
            let id = row.value().attr("data-id").unwrap();
            manga.path = format!("/api/manga/{}", id);

            let sel = scraper::Selector::parse("div a img").unwrap();
            for el in row.select(&sel) {
                manga.thumbnail_url = format!("{}{}", url, el.value().attr("src").unwrap().to_owned());
            }

            let sel = scraper::Selector::parse(".manga_title").unwrap();
            for el in row.select(&sel) {
                manga.title = el.inner_html();
            }
            mangas.push(manga);
        }

        GetMangasResponse { mangas }
    }

    fn get_manga_info(url: &String) -> GetMangaResponse {
        let resp = ureq::get(url.as_str()).call();
        let mangadex_resp: tanoshi::mangadex::GetMangaResponse = serde_json::from_reader(resp.into_reader()).unwrap();

        let description_split = mangadex_resp.manga.description.split("\r\n").collect::<Vec<_>>();
        let description = match description_split[0].to_string().starts_with("[b][u]") {
            true => description_split[1].to_string(),
            false => description_split[0].to_string(),
        };
        let m = Manga {
            id: 0,
            title: mangadex_resp.manga.title,
            author: mangadex_resp.manga.author,
            //genre: vec![],
            status: match mangadex_resp.manga.status {
                1 => "Ongoing".to_string(),
                2 => "Completed".to_string(),
                3 => "Cancelled".to_string(),
                4 => "Hiatus".to_string(),
                _ => "Ongoing".to_string(),
            },
            description: description,
            path: "".to_string(),
            thumbnail_url: format!("https://mangadex.org{}", mangadex_resp.manga.cover_url),
            last_read: None,
            last_page: None,
            is_favorite: false,
        };

        GetMangaResponse { manga: m }
    }

    fn get_chapters(url: &String) -> GetChaptersResponse {
        let mut chapters: Vec<Chapter> = Vec::new();

        let resp = ureq::get(url.as_str()).call();
        let mangadex_resp: tanoshi::mangadex::GetChapterResponse = serde_json::from_reader(resp.into_reader()).unwrap();

        for (id, chapter) in mangadex_resp.chapter {
            if chapter.lang_code == "gb".to_string() {
                chapters.push(Chapter{
                    id: 0,
                    manga_id: 0,
                    no: match chapter.chapter.as_str() {
                        "" => "0".to_string(),
                        _ => chapter.chapter,
                    },
                    title: chapter.title,
                    url: format!("/api/chapter/{}", id),
                    read: 0,
                    uploaded: chrono::NaiveDateTime::from_timestamp(chapter.timestamp, 0),
                })
            }
        }

        GetChaptersResponse { chapters }
    }

    fn get_pages(url: &String) -> GetPagesResponse {
        let mut pages = Vec::new();

        let resp = ureq::get(url.as_str()).call();
        let mangadex_resp: tanoshi::mangadex::GetPagesResponse = serde_json::from_reader(resp.into_reader()).unwrap();

        for page in mangadex_resp.page_array {
            pages.push(format!("{}{}/{}", mangadex_resp.server, mangadex_resp.hash, page));
        }

        GetPagesResponse { 
            manga_id: 0,
            pages 
        }
    }
}

impl Mangadex {
    pub fn login(url: &String, login: MangadexLogin) -> Result<Vec<String>, String> {
        let boundary = "__TANOSHI__";
        let body = format!(
r#"--{}
Content-Disposition: form-data; name="login_username"

{}
--{}
Content-Disposition: form-data; name="login_password"

{}
--{}
Content-Disposition: form-data; name="remember_me"

{}
--{}
Content-Disposition: form-data; name="two_factor"

{}
--{}--"#,
            boundary,
            login.login_username,
            boundary,
            login.login_password,
            boundary,
            login.remember_me as i32,
            boundary,
            login.two_factor,
            boundary);

        let resp = ureq::post(format!("{}/ajax/actions.ajax.php?function=login", url).as_str())
            .set("X-Requested-With", "XMLHttpRequest")
            .set("Content-Type", format!("multipart/form-data; charset=utf-8; boundary={}", boundary).as_str())
            .set("User-Agent", "Tanoshi/0.1.0")
            .send_string(&body);

        let cookies = resp.all("Set-Cookie").into_iter().map(|c| c.to_string()).collect();
        Ok(cookies)
    }

}