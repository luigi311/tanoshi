use anyhow::Result;

pub mod repository;
pub mod mangasee;
pub mod mangadex;
pub mod local;


use tanoshi::manga::{
    Manga, Chapter, Params
};

pub trait Scraping {
    fn get_mangas(url: &String, param: Params, cookies: Vec<String>) -> Result<Vec<Manga>> ;
    fn get_manga_info(url: &String) -> Result<Manga>;
    fn get_chapters(url: &String) -> Result<Vec<Chapter>>;
    fn get_pages(url: &String) -> Result<Vec<String>>;
}
