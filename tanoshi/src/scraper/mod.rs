use chrono::Local;
use serde::{Deserialize, Serialize};

pub mod repository;
pub mod mangasee;
use mangasee::Mangasee;

pub mod mangadex;

use tanoshi::manga::{
    GetMangasResponse, GetMangaResponse, GetChaptersResponse, GetPagesResponse, Params
};

pub trait Scraping {
    fn get_mangas(url: &String, param: Params, cookies: Vec<String>) -> GetMangasResponse;
    fn get_manga_info(url: &String) -> GetMangaResponse;
    fn get_chapters(url: &String) -> GetChaptersResponse;
    fn get_pages(url: &String) -> GetPagesResponse;
}
