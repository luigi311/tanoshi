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
            SortOrderParam::Asc => s + 1,
            SortOrderParam::Desc => s,
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

            let sel = scraper::Selector::parse("div a img").unwrap();
            for el in row.select(&sel) {
                manga.thumbnail_url = format!("{}{}", url, el.value().attr("src").unwrap().to_owned());
            }

            let sel = scraper::Selector::parse(".manga_title").unwrap();
            for el in row.select(&sel) {
                manga.title = el.inner_html();
                manga.path = el.value().attr("href").unwrap().to_owned();
            }
            mangas.push(manga);
        }

        GetMangasResponse { mangas }
    }

    fn get_manga_info(url: &String) -> GetMangaResponse {
        let mut m = Manga {
            title: "".to_string(),
            author: "".to_string(),
            //genre: vec![],
            status: "".to_string(),
            description: "".to_string(),
            path: "".to_string(),
            thumbnail_url: "".to_string(),
            last_read: None,
            last_page: None,
            is_favorite: false,
        };

        let resp = ureq::get(url.as_str()).call();
        let html = resp.into_string().unwrap();

        let document = scraper::Html::parse_document(&html);

        let selector = scraper::Selector::parse(".leftImage img").unwrap();
        for element in document.select(&selector) {
            let src = element.value().attr("src").unwrap();
            m.thumbnail_url = String::from(src);
        }

        let selector = scraper::Selector::parse("h1[class=\"SeriesName\"]").unwrap();
        for element in document.select(&selector) {
            m.title = element.inner_html();
        }

        let selector = scraper::Selector::parse("a[href*=\"author\"]").unwrap();

        for element in document.select(&selector) {
            for text in element.text() {
                m.author = String::from(text);
            }
        }

        let selector = scraper::Selector::parse("a[href*=\"genre\"]").unwrap();
        for element in document.select(&selector) {
            for text in element.text() {
                //m.genre.push(String::from(text));
            }
        }

        let selector = scraper::Selector::parse(".PublishStatus").unwrap();
        for element in document.select(&selector) {
            let status = element.value().attr("status").unwrap();
            m.status = String::from(status);
        }

        let selector = scraper::Selector::parse(".description").unwrap();
        for element in document.select(&selector) {
            for text in element.text() {
                m.description = String::from(text);
            }
        }

        GetMangaResponse { manga: m }
    }

    fn get_chapters(url: &String) -> GetChaptersResponse {
        let mut chapters: Vec<Chapter> = Vec::new();
        let resp = ureq::get(url.as_str()).call();
        let html = resp.into_string().unwrap();

        let document = scraper::Html::parse_document(&html);
        let selector = scraper::Selector::parse(".mainWell .chapter-list a[chapter]").unwrap();
        for element in document.select(&selector) {
            let mut chapter = Chapter::default();

            chapter.no = String::from(element.value().attr("chapter").unwrap());

            let link = element.value().attr("href").unwrap();
            chapter.url = link.replace("-page-1", "");

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