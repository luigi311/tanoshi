use regex::Regex;
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
            SortByParam::Views => 4,
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

        let description = mangadex_resp.manga.description.split("\r\n").collect::<Vec<_>>();
        let mut m = Manga {
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
            description: description[0].to_string(),
            path: "".to_string(),
            thumbnail_url: mangadex_resp.manga.cover_url,
            last_read: None,
            last_page: None,
            is_favorite: false,
        };

        GetMangaResponse { manga: m }
    }

    fn get_chapters(url: &String) -> GetChaptersResponse {
        let mut chapters: Vec<Chapter> = Vec::new();
        let resp = ureq::get(url.as_str()).call();
        let html = resp.into_string().unwrap();

        let document = scraper::Html::parse_document(&html);
        let selector = scraper::Selector::parse(".chapter-container .chapter-row .col.row a").unwrap();
        for element in document.select(&selector) {
            let mut chapter = Chapter::default();

            for text in element.text() {
                chapter.no = String::from(text);
            }
            let splitted = chapter.no.split_ascii_whitespace().collect::<Vec<_>>();
            println!("{:?}", splitted.clone());
            let idx = splitted.clone().into_iter().position(|p| p == "Ch.").unwrap();
            chapter.no = splitted[idx+1].to_string();
            chapter.url = element.value().attr("href").unwrap().to_string();

            let time_sel = scraper::Selector::parse("time[class*=\"SeriesTime\"]").unwrap();
            for time_el in element.select(&time_sel) {
                let date_str = time_el.value().attr("datetime").unwrap();
                chapter.uploaded = chrono::NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%dT%H:%M:%S%:z").unwrap()
            }

            chapters.push(chapter);
        }

        GetChaptersResponse { chapters }
    }

    fn get_pages(url: &String) -> GetPagesResponse {
        let mut pages = Vec::new();
        let resp = ureq::get(url.as_str()).call();
        let html = resp.into_string().unwrap();

        let document = scraper::Html::parse_document(&html);

        let selector = scraper::Selector::parse(".fullchapimage img").unwrap();
        for element in document.select(&selector) {
            pages.push(String::from(element.value().attr("src").unwrap()));
        }
        GetPagesResponse { pages }
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