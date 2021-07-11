use std::{fs::DirEntry, path::PathBuf, time::UNIX_EPOCH};

use chrono::NaiveDateTime;
use fancy_regex::Regex;
use libarchive_rs;
use tanoshi_lib::prelude::{Chapter, Extension, ExtensionResult, Manga, Source};

pub static ID: i64 = 1;

pub struct Local {
    path: String,
}

impl Local {
    pub fn new(path: String) -> Box<dyn Extension> {
        Box::new(Self { path })
    }

    fn default_cover_url() -> String {
        "/images/cover-placeholder.jpg".to_string()
    }

    fn filter_file_only(entry: Result<DirEntry, std::io::Error>) -> Option<DirEntry> {
        match entry {
            Ok(res) => {
                if res.path().is_file() {
                    Some(res)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn find_cover_url(entry: &PathBuf) -> String {
        let entry_read_dir = match entry.read_dir() {
            Ok(entry_read_dir) => entry_read_dir,
            Err(e) => {
                return Self::default_cover_url();
            }
        };

        let dir_entry = match entry_read_dir
            .into_iter()
            .filter_map(Self::filter_file_only)
            .next()
        {
            Some(entry) => entry,
            None => {
                return Self::default_cover_url();
            }
        };
        let path = dir_entry.path();
        match libarchive_rs::list_archive_files(format!("{}", path.display()).as_str()) {
            Ok(files) => files
                .first()
                .map(|first_page| format!("{}", path.join(first_page).display()))
                .unwrap_or_else(Self::default_cover_url),
            Err(_) => Self::default_cover_url(),
        }
    }
}

impl Extension for Local {
    fn detail(&self) -> Source {
        Source {
            id: ID,
            name: "local".to_string(),
            url: self.path.clone(),
            version: "1.0.0".to_string(),
            icon: "/icons/192.png".to_string(),
            need_login: false,
        }
    }

    fn get_manga_list(&self, param: tanoshi_lib::prelude::Param) -> ExtensionResult<Vec<Manga>> {
        let read_dir = match std::fs::read_dir(&self.path) {
            Ok(read_dir) => read_dir,
            Err(e) => {
                return ExtensionResult::err(format!("{}", e).as_str());
            }
        };

        let mut data = vec![];
        for entry in read_dir.into_iter().filter_map(|entry| entry.ok()) {
            data.push(Manga {
                source_id: ID,
                title: entry.file_name().to_str().unwrap_or("").to_string(),
                author: vec![],
                genre: vec![],
                status: None,
                description: None,
                path: entry.path().to_str().unwrap_or("").to_string(),
                cover_url: Self::find_cover_url(&entry.path()),
            })
        }

        ExtensionResult::ok(data)
    }

    fn get_manga_info(&self, path: String) -> ExtensionResult<Manga> {
        let path = PathBuf::from(&path);

        let cover_url = Self::find_cover_url(&path);

        let title = path
            .file_name()
            .and_then(|filename| filename.to_str())
            .unwrap_or("")
            .to_string();
        ExtensionResult::ok(Manga {
            source_id: ID,
            title: title.clone(),
            author: vec![],
            genre: vec![],
            status: Some("".to_string()),
            description: Some(title),
            path: path.to_str().unwrap_or("").to_string(),
            cover_url,
        })
    }

    fn get_chapters(&self, path: String) -> ExtensionResult<Vec<Chapter>> {
        let read_dir = match std::fs::read_dir(&path) {
            Ok(read_dir) => read_dir,
            Err(e) => {
                return ExtensionResult::err(format!("{}", e).as_str());
            }
        };

        let mut data = vec![];
        for entry in read_dir.into_iter().filter_map(|entry| entry.ok()) {
            let modified = match entry
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            {
                Some(modified) => modified.as_secs(),
                None => {
                    return ExtensionResult::err("failed to get modified date");
                }
            };
            let number_re = match Regex::new(
                r"(?i)(?<=v)(\d+)|(?<=volume)\s*(\d+)|(?<=vol)\s*(\d+)|(?<=\s)(\d+)",
            ) {
                Ok(re) => re,
                Err(e) => {
                    return ExtensionResult::err(format!("failed to create regex: {}", e).as_str());
                }
            };
            let file_name = match entry
                .path()
                .file_stem()
                .and_then(|file_stem| file_stem.to_str())
            {
                Some(file_stem) => file_stem.to_string(),
                None => {
                    return ExtensionResult::err("no filestem");
                }
            };
            let number = match number_re.find(&file_name).ok().and_then(|m| m) {
                Some(mat) => mat.as_str().parse().unwrap_or(0_f64),
                None => 10000_f64,
            };

            data.push(Chapter {
                source_id: ID,
                title: file_name,
                path: format!("{}", entry.path().display()),
                number,
                scanlator: "".to_string(),
                uploaded: NaiveDateTime::from_timestamp(modified as i64, 0),
            })
        }

        ExtensionResult::ok(data)
    }

    fn get_pages(&self, filename: String) -> ExtensionResult<Vec<String>> {
        let path = PathBuf::from(filename.clone());
        match libarchive_rs::list_archive_files(&filename) {
            Ok(files) => {
                let pages = files
                    .into_iter()
                    .map(|p| format!("{}", path.clone().join(p).display()))
                    .collect();
                ExtensionResult::ok(pages)
            }
            Err(e) => ExtensionResult::err(format!("{}", e).as_str()),
        }
    }
}
