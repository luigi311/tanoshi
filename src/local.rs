use std::{
    fs::DirEntry,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use chrono::NaiveDateTime;
use fancy_regex::Regex;
use phf::phf_set;
use tanoshi_lib::prelude::{Chapter, Extension, ExtensionResult, Filters, Manga, Source};

pub static ID: i64 = 1;
// list of supported files, other archive may works but no tested
static SUPPORTED_FILES: phf::Set<&'static str> = phf_set! {
    "cbz",
    "cbr",
};

pub struct Local {
    path: PathBuf,
}

impl Local {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = PathBuf::new().join(path);
        Self { path }
    }

    fn default_cover_url() -> String {
        "/images/cover-placeholder.jpg".to_string()
    }

    fn filter_supported_files_and_folders(
        entry: Result<DirEntry, std::io::Error>,
    ) -> Option<DirEntry> {
        let entry = entry.ok()?;
        if entry.file_type().ok()?.is_dir() {
            Some(entry)
        } else if let Some(ext) = entry.path().extension() {
            if ext
                .to_str()
                .map(|ext| SUPPORTED_FILES.contains(&ext.to_lowercase()))
                .unwrap_or(false)
            {
                Some(entry)
            } else {
                None
            }
        } else {
            None
        }
    }

    // find first image from an archvie
    fn find_cover_from_archive(path: &PathBuf) -> String {
        match libarchive_rs::list_archive_files(format!("{}", path.display()).as_str()) {
            Ok(files) => files
                .first()
                .map(|first_page| format!("{}", path.join(first_page).display()))
                .unwrap_or_else(Self::default_cover_url),
            Err(_) => Self::default_cover_url(),
        }
    }

    // find first image from a directory
    fn find_cover_from_dir(path: &PathBuf) -> String {
        match path.read_dir() {
            Ok(dir) => {
                if let Some(Ok(f)) = dir.into_iter().next() {
                    f.path().display().to_string()
                } else {
                    Self::default_cover_url()
                }
            }
            Err(_) => return Self::default_cover_url(),
        }
    }

    fn find_cover_url(entry: &Path) -> String {
        let entry_read_dir = match entry.read_dir() {
            Ok(entry_read_dir) => entry_read_dir,
            Err(_) => {
                return Self::default_cover_url();
            }
        };

        let dir_entry = match entry_read_dir
            .into_iter()
            .find_map(Self::filter_supported_files_and_folders)
        {
            Some(entry) => entry,
            None => {
                return Self::default_cover_url();
            }
        };

        let path = dir_entry.path();
        if path.is_dir() {
            Self::find_cover_from_dir(&path)
        } else if path.is_file() {
            Self::find_cover_from_archive(&path)
        } else {
            Self::default_cover_url()
        }
    }

    fn get_pages_from_archive(
        path: &PathBuf,
        filename: String,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match libarchive_rs::list_archive_files(&filename) {
            Ok(files) => {
                let pages = files
                    .into_iter()
                    .map(|p| format!("{}", path.clone().join(p).display()))
                    .collect();
                Ok(pages)
            }
            Err(e) => Err(e),
        }
    }

    fn get_pages_from_dir(path: &PathBuf) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let pages = path
            .read_dir()?
            .into_iter()
            .filter_map(|f| {
                if let Ok(f) = f.map(|f| f.path()) {
                    if f.is_file() {
                        Some(f.display().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(pages)
    }
}

impl Extension for Local {
    fn detail(&self) -> Source {
        Source {
            id: ID,
            name: "local".to_string(),
            url: format!("{}", self.path.display()),
            version: "1.0.0".to_string(),
            icon: "/icons/192.png".to_string(),
            need_login: false,
            languages: vec![],
        }
    }

    fn filters(&self) -> ExtensionResult<Option<Filters>> {
        ExtensionResult::ok(None)
    }

    fn get_manga_list(&self, _param: tanoshi_lib::prelude::Param) -> ExtensionResult<Vec<Manga>> {
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
        if path.is_dir() {
            match Self::get_pages_from_dir(&path) {
                Ok(pages) => ExtensionResult::ok(pages),
                Err(e) => ExtensionResult::err(format!("{}", e).as_str()),
            }
        } else if path.is_file() {
            match Self::get_pages_from_archive(&path, filename) {
                Ok(pages) => ExtensionResult::ok(pages),
                Err(e) => ExtensionResult::err(format!("{}", e).as_str()),
            }
        } else {
            ExtensionResult::err("filename neither file or dir")
        }
    }
}
