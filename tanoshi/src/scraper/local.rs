use crate::scraper::Scraping;
use human_sort::compare;
use std::{fs, io};
use std::io::{BufReader, Read};
use tanoshi::manga::{
    Chapter, GetChaptersResponse, GetMangaResponse, GetMangasResponse, GetPagesResponse, GetParams,
    Manga, Params, SortByParam, SortOrderParam,
};
use zip::result::ZipError;
use fancy_regex::Regex;
use anyhow::Result;

#[derive(Clone)]
pub struct Local {
    pub path: String,
}

impl Scraping for Local {
    fn get_mangas(url: &String, param: Params, cookies: Vec<String>) -> Result<Vec<Manga>> {
        let entries = fs::read_dir(url)
            .expect("error read directory")
            .filter(|res| res.as_ref().unwrap().file_type().unwrap().is_dir())
            .map(|res| {
                res.map(|e| Manga {
                    id: 0,
                    title: e.file_name().to_str().unwrap().to_string(),
                    author: "".to_string(),
                    //genre: vec![],
                    status: "".to_string(),
                    description: "".to_string(),
                    path: e.path().to_str().unwrap().to_string(),
                    thumbnail_url: "".to_string(),
                    last_read: None,
                    last_page: None,
                    is_favorite: false,
                })
            })
            .collect::<Result<Vec<_>, io::Error>>()
            .unwrap_or(vec![]);

        Ok(entries)
    }

    fn get_manga_info(url: &String) -> Result<Manga> {
        Ok(Manga::default())
    }

    fn get_chapters(url: &String) -> Result<Vec<Chapter>> {
        let re = Regex::new(r"(?<=v)(\d+)|(?<=volume)\s*(\d+)|(?<=vol)\s*(\d+)|(?<=ch)(\d+)|(?<=chapter)\s*(\d+)|(\d+)").unwrap();
        let entries = fs::read_dir(url)
            .expect("error read directory")
            .filter(|res| res.as_ref().unwrap().file_type().unwrap().is_file() 
            && !res.as_ref().unwrap().file_name().as_os_str().to_str().unwrap().starts_with("."))
            .map(|res| {
                res.map(|e| {
                    let mut ch = Chapter::default();
                    let file_name = e.file_name().to_str().unwrap().to_string();
                    let mat = re.find(file_name.as_str()).unwrap();
                    ch.no = mat.unwrap().as_str().to_string();
                    ch.title = file_name;
                    ch.url = e.path().to_str().unwrap().to_string();
                    ch
                })
            })
            .collect::<Result<Vec<_>, io::Error>>()
            .unwrap_or(vec![]);

        Ok(entries)
    }

    fn get_pages(url: &String) -> Result<Vec<String>> {
        let file = fs::File::open(&url).unwrap();
        let reader = BufReader::new(file);

        let archive = zip::ZipArchive::new(reader).unwrap();
        let mut pages: Vec<String> = archive.file_names().map(|file_name| format!("{}/{}", url, file_name)).collect();
        pages.sort();
        Ok(pages)
    }
}

impl Local {
    pub fn get_page(url: &String, bytes: &mut Vec<u8>) -> Result<String, ZipError>  {
        let path = std::path::Path::new(url);
        let dir = path.parent().unwrap().to_str().unwrap();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let file_ext = path.extension().unwrap().to_str().unwrap();

        let file = fs::File::open(&dir)?;
        let reader = BufReader::new(file);

        let mut archive = zip::ZipArchive::new(reader)?;
        let mut zip_file = archive.by_name(file_name)?;
        io::copy(&mut zip_file, bytes)?;
        Ok(file_ext.to_string())
    }
}