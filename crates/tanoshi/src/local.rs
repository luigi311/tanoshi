use std::{
    ffi::OsStr,
    fs::{DirEntry, ReadDir},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fancy_regex::Regex;
use serde::{Deserialize, Serialize};
use tanoshi_lib::prelude::{ChapterInfo, Extension, Input, Lang, MangaInfo, SourceInfo};

pub static ID: i64 = 1;
// list of supported files, other archive may works but no tested
static SUPPORTED_FILES: phf::Set<&'static str> = phf::phf_set! {
    "cbz",
    "cbr",
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalMangaInfo {
    pub title: Option<String>,
    pub author: Option<Vec<String>>,
    pub genre: Option<Vec<String>>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub cover_path: Option<String>,
}

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
        entry.ok().and_then(|entry| {
            if entry.path().is_dir() {
                Some(entry)
            } else {
                entry
                    .path()
                    .extension()?
                    .to_str()
                    .map(|ext| SUPPORTED_FILES.contains(&ext.to_lowercase()))
                    .and_then(|supported| if supported { Some(entry) } else { None })
            }
        })
    }

    // find first image from an archvie
    fn find_cover_from_archive(path: &Path) -> String {
        let source = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) => {
                error!("error open {}, reason {}", path.display(), e);
                return Self::default_cover_url();
            }
        };

        compress_tools::list_archive_files(source)
            .ok()
            .and_then(|files| files.first().cloned())
            .map(|page| path.join(page).display().to_string())
            .unwrap_or_else(Self::default_cover_url)
    }

    // find first image from a directory
    fn find_cover_from_dir(path: &Path) -> String {
        path.read_dir()
            .ok()
            .map(Self::sort_dir)
            .and_then(|dir| dir.into_iter().next())
            .map(|entry| entry.path().display().to_string())
            .unwrap_or_else(Self::default_cover_url)
    }

    // find details from an archvie
    fn find_details_from_archive(path: &Path) -> Option<Vec<u8>> {
        if let Ok(source) = std::fs::File::open(path) {
            let mut data = vec![];
            if compress_tools::uncompress_archive_file(source, &mut data, "details.json").is_ok() {
                return Some(data);
            }
        }

        None
    }

    // find first image from a directory
    fn find_details_from_dir(path: &Path) -> Option<Vec<u8>> {
        std::fs::read(path.join("details.json")).ok()
    }

    fn sort_dir(dir: ReadDir) -> Vec<DirEntry> {
        Self::sort_read_dir_with_reverse(dir, false)
    }

    #[allow(dead_code)]
    fn sort_dir_reverse(dir: ReadDir) -> Vec<DirEntry> {
        Self::sort_read_dir_with_reverse(dir, true)
    }

    fn sort_read_dir_with_reverse(dir: ReadDir, reverse: bool) -> Vec<DirEntry> {
        let mut dir: Vec<DirEntry> = dir.into_iter().filter_map(Result::ok).collect();
        dir.sort_by(|a, b| {
            human_sort::compare(
                a.path().display().to_string().as_str(),
                b.path().display().to_string().as_str(),
            )
        });
        if reverse {
            dir.reverse();
        }
        dir
    }

    fn find_cover_url(entry: &Path) -> String {
        if entry.is_file() {
            return Self::find_cover_from_archive(entry);
        }

        let entry_read_dir = match entry.read_dir() {
            Ok(entry_read_dir) => entry_read_dir,
            Err(_) => {
                return Self::default_cover_url();
            }
        };

        let path = match entry_read_dir
            .into_iter()
            .find_map(Self::filter_supported_files_and_folders)
        {
            Some(entry) => entry.path(),
            None => {
                return Self::default_cover_url();
            }
        };

        if path.is_dir() {
            Self::find_cover_from_dir(&path)
        } else if path.is_file() {
            Self::find_cover_from_archive(&path)
        } else {
            Self::default_cover_url()
        }
    }

    fn find_details(path: &Path) -> Option<Vec<u8>> {
        if path.is_dir() {
            Self::find_details_from_dir(path)
        } else if path.is_file() {
            Self::find_details_from_archive(path)
        } else {
            None
        }
    }

    pub fn get_pages_from_archive(path: &Path) -> Result<Vec<String>, anyhow::Error> {
        let source = std::fs::File::open(path)?;
        match compress_tools::list_archive_files(source) {
            Ok(files) => {
                let pages = files
                    .into_iter()
                    .map(|p| path.join(p).display().to_string())
                    .collect();
                Ok(pages)
            }
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn get_pages_from_dir(path: &Path) -> Result<Vec<String>, anyhow::Error> {
        let pages = path
            .read_dir()?
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|f| (f.path().is_file()).then(|| f.path().display().to_string()))
            .collect();
        Ok(pages)
    }

    fn map_entry_to_chapter(path: &Path) -> Option<ChapterInfo> {
        let modified = match path
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        {
            Some(modified) => modified.as_secs(),
            None => {
                return None;
            }
        };
        let number_re = match Regex::new(
            r"(?i)(?<=v)(\d+)|(?<=volume)\s*(\d+)|(?<=vol)\s*(\d+)|(?<=\s)(\d+)|(\d+)",
        ) {
            Ok(re) => re,
            Err(_) => {
                return None;
            }
        };
        let file_name = match path.file_stem().and_then(|file_stem| file_stem.to_str()) {
            Some(file_stem) => file_stem.to_string(),
            None => {
                return None;
            }
        };
        let number = match number_re.find(&file_name).ok().and_then(|m| m) {
            Some(mat) => mat.as_str().parse().unwrap_or(0_f64),
            None => 10000_f64,
        };

        Some(ChapterInfo {
            source_id: ID,
            title: file_name,
            path: format!("{}", path.display()),
            number,
            scanlator: None,
            uploaded: modified as i64,
        })
    }
}

#[async_trait]
impl Extension for Local {
    fn get_source_info(&self) -> SourceInfo {
        SourceInfo {
            id: ID,
            name: "local".to_string(),
            url: format!("{}", self.path.display()),
            version: "0.0.0".to_string(),
            icon: "/icons/192.png".to_string(),
            languages: Lang::All,
            nsfw: false,
        }
    }

    fn get_filter_list(&self) -> Result<Vec<Input>> {
        Ok(vec![])
    }

    fn get_preferences(&self) -> Result<Vec<Input>> {
        Ok(vec![])
    }

    fn set_preferences(&self, _: Vec<Input>) -> Result<()> {
        Ok(())
    }

    async fn get_popular_manga(&self, page: i64) -> Result<Vec<MangaInfo>> {
        self.search_manga(page, None, None).await
    }

    async fn get_latest_manga(&self, _page: i64) -> Result<Vec<MangaInfo>> {
        Ok(vec![])
    }

    async fn search_manga(
        &self,
        page: i64,
        query: Option<String>,
        _filters: Option<Vec<Input>>,
    ) -> Result<Vec<MangaInfo>> {
        // let page = param.page.map(|p| p as usize).unwrap_or(1);
        let offset = (page - 1) * 20;

        let read_dir = match std::fs::read_dir(&self.path) {
            Ok(read_dir) => read_dir,
            Err(e) => {
                return Err(anyhow!("{}", e));
            }
        };

        let mut data: Box<dyn Iterator<Item = _>> = Box::new(
            read_dir
                .into_iter()
                .filter_map(Self::filter_supported_files_and_folders),
        );

        if let Some(keyword) = query {
            data = Box::new(data.filter(move |entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|a| a.to_lowercase().contains(&keyword))
                    .unwrap_or_else(|| false)
            }));
        }

        let manga = data
            .skip(offset as _)
            .take(20)
            .map(|entry| MangaInfo {
                source_id: ID,
                title: entry
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string(),
                author: vec![],
                genre: vec![],
                status: None,
                description: None,
                path: entry.path().to_str().unwrap_or("").to_string(),
                cover_url: Self::find_cover_url(&entry.path()),
            })
            .collect::<Vec<_>>();

        Ok(manga)
    }

    async fn get_manga_detail(&self, path: String) -> Result<MangaInfo> {
        let path = PathBuf::from(path);

        let title = path
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();
        let cover_url = Self::find_cover_url(&path);

        let mut manga = MangaInfo {
            source_id: ID,
            title: title.clone(),
            author: vec![],
            genre: vec![],
            status: Some("".to_string()),
            description: Some(title),
            path: path.display().to_string(),
            cover_url,
        };

        if let Some(info) = Self::find_details(&path)
            .and_then(|object| serde_json::from_slice::<LocalMangaInfo>(&object).ok())
        {
            if let Some(title) = info.title {
                manga.title = title;
            }
            if let Some(cover_path) = info.cover_path {
                manga.cover_url = path.join(cover_path).display().to_string();
            }
            if let Some(author) = info.author {
                manga.author = author;
            }
            if let Some(genre) = info.genre {
                manga.genre = genre;
            }
            if let Some(description) = info.description {
                manga.description = Some(description);
            }
        }

        Ok(manga)
    }

    async fn get_chapters(&self, path: String) -> Result<Vec<ChapterInfo>> {
        let path = PathBuf::from(path);
        if path.is_file() {
            if let Some(data) = Self::map_entry_to_chapter(&path) {
                return Ok(vec![data]);
            }
        }

        let read_dir = match std::fs::read_dir(&path) {
            Ok(read_dir) => read_dir,
            Err(e) => {
                return Err(anyhow!("{}", e));
            }
        };

        let mut data: Vec<ChapterInfo> = read_dir
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|entry| Self::map_entry_to_chapter(&entry.path()))
            .collect();

        data.sort_by(|a, b| a.number.partial_cmp(&b.number).unwrap());
        data.reverse();
        Ok(data)
    }

    async fn get_pages(&self, filename: String) -> Result<Vec<String>> {
        let path = PathBuf::from(filename.clone());
        let mut pages = if path.is_dir() {
            match Self::get_pages_from_dir(&path) {
                Ok(pages) => pages,
                Err(e) => return Err(anyhow!("{}", e)),
            }
        } else if path.is_file() {
            match Self::get_pages_from_archive(&path) {
                Ok(pages) => pages,
                Err(e) => return Err(anyhow!("{}", e)),
            }
        } else {
            return Err(anyhow!("filename neither file or dir"));
        };

        pages.sort_by(|a, b| human_sort::compare(a, b));

        Ok(pages)
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, iter::FromIterator};

    use super::*;

    #[tokio::test]
    async fn test_positive_get_popular_manga() {
        let local = Local::new("../../test/data/manga");
        let manga = local.get_popular_manga(1).await;

        if let Ok(data) = manga {
            assert_eq!(data.len(), 3);

            let path_set: HashSet<String> = HashSet::from_iter(data.iter().map(|a| a.path.clone()));
            #[cfg(target_family = "windows")]
            let want_path_set = HashSet::from_iter(vec![
                "../../test/data/manga\\Space_Adventures_004__c2c__diff_ver.cbz".to_string(),
                "../../test/data/manga\\Space Adventures".to_string(),
                "../../test/data/manga\\Super Duck".to_string(),
            ]);
            #[cfg(target_family = "unix")]
            let want_path_set = HashSet::from_iter(vec![
                "../../test/data/manga/Space_Adventures_004__c2c__diff_ver.cbz".to_string(),
                "../../test/data/manga/Space Adventures".to_string(),
                "../../test/data/manga/Super Duck".to_string(),
            ]);

            assert_eq!(path_set, want_path_set);

            let title_set: HashSet<String> =
                HashSet::from_iter(data.iter().map(|a| a.title.clone()));
            let want_title_set = HashSet::from_iter(vec![
                "Space Adventures".to_string(),
                "Space_Adventures_004__c2c__diff_ver".to_string(),
                "Super Duck".to_string(),
            ]);

            assert_eq!(title_set, want_title_set);
        }
    }

    #[tokio::test]
    async fn test_negative_get_popular_manga() {
        let local = Local::new("../../test/data/not_manga");
        let manga = local.get_popular_manga(1).await;

        assert!(manga.is_err());
    }

    #[tokio::test]
    async fn test_positive_get_popular_manga_with_page() {
        let local = Local::new("../../test/data/manga");
        let manga = local.get_popular_manga(2).await;

        assert!(manga.is_ok());
        assert_eq!(manga.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_get_manga_detail_single_archive() {
        let local = Local::new("../../test/data/manga");
        #[cfg(target_family = "windows")]
        let manga = local
            .get_manga_detail(
                "../../test/data/manga\\Space_Adventures_004__c2c__diff_ver.cbz".to_string(),
            )
            .await;
        #[cfg(target_family = "unix")]
        let manga = local
            .get_manga_detail(
                "../../test/data/manga/Space_Adventures_004__c2c__diff_ver.cbz".to_string(),
            )
            .await;

        assert!(manga.is_ok());

        if let Ok(data) = manga {
            assert_eq!(data.source_id, 1);
            assert_eq!(data.title, "Space_Adventures_004__c2c__diff_ver");
            #[cfg(target_family = "windows")]
            assert_eq!(
                data.cover_url,
                "../../test/data/manga\\Space_Adventures_004__c2c__diff_ver.cbz\\SPA00401.JPG"
            );
            #[cfg(target_family = "unix")]
            assert_eq!(
                data.cover_url,
                "../../test/data/manga/Space_Adventures_004__c2c__diff_ver.cbz/SPA00401.JPG"
            );
        }
    }

    #[tokio::test]
    async fn test_get_manga_detail() {
        let local = Local::new("../../test/data/manga");
        #[cfg(target_family = "windows")]
        let manga = local
            .get_manga_detail("../../test/data/manga\\Super Duck".to_string())
            .await;
        #[cfg(target_family = "unix")]
        let manga = local
            .get_manga_detail("../../test/data/manga/Super Duck".to_string())
            .await;

        assert!(manga.is_ok());

        if let Ok(data) = manga {
            assert_eq!(data.source_id, 1);
            assert_eq!(data.title, "Super Duck");
            assert_eq!(data.description, Some("Super Duck is the greatest hero of Ducktropolis. Brash, arrogant and virtually unbeatable, he’s defeated all threats to the city and routinely foils the schemes of his greatest rival, criminal genius and corporate billionaire Dapper Duck. But now, three years later, Super Duck has fallen on hard times. Down on his luck and with his superheroing days a distant memory, he is reduced to appearing at comic conventions for measly appearance fees. So when he’s approached by a rival of Dapper to be his personal bodyguard/accompany him on his many adventures, Supe has to decide if he’s ready to don his cape once more in this series for mature readers!".to_string()));
            #[cfg(target_family = "windows")]
            assert_eq!(
                data.cover_url,
                "../../test/data/manga\\Super Duck\\super_duck_1/duck01.jpg"
            );
            #[cfg(target_family = "unix")]
            assert_eq!(
                data.cover_url,
                "../../test/data/manga/Super Duck/super_duck_1/duck01.jpg"
            );
        }
    }

    #[tokio::test]
    async fn test_single_chapter_manga_get_chapters() {
        let local = Local::new("../../test/data/manga");
        #[cfg(target_family = "windows")]
        let chapter = local
            .get_chapters(
                "../../test/data/manga\\Space_Adventures_004__c2c__diff_ver.cbz".to_string(),
            )
            .await;
        #[cfg(target_family = "unix")]
        let chapter = local
            .get_chapters(
                "../../test/data/manga/Space_Adventures_004__c2c__diff_ver.cbz".to_string(),
            )
            .await;

        assert!(chapter.is_ok());

        if let Ok(data) = chapter {
            assert_eq!(data.len(), 1);

            assert_eq!(data[0].source_id, 1);
            assert_eq!(data[0].title, "Space_Adventures_004__c2c__diff_ver");
            #[cfg(target_family = "windows")]
            assert_eq!(
                data[0].path,
                "../../test/data/manga\\Space_Adventures_004__c2c__diff_ver.cbz"
            );
            #[cfg(target_family = "unix")]
            assert_eq!(
                data[0].path,
                "../../test/data/manga/Space_Adventures_004__c2c__diff_ver.cbz"
            );
        }
    }

    #[tokio::test]
    async fn test_manga_get_chapters() {
        let local = Local::new("../../test/data/manga");
        #[cfg(target_family = "windows")]
        let chapter = local
            .get_chapters("../../test/data/manga\\Space Adventures".to_string())
            .await;
        #[cfg(target_family = "unix")]
        let chapter = local
            .get_chapters("../../test/data/manga/Space Adventures".to_string())
            .await;

        assert!(chapter.is_ok());

        if let Ok(data) = chapter {
            assert_eq!(data.len(), 2);

            assert_eq!(data[0].source_id, 1);
            assert_eq!(data[0].number, 4.0_f64);
            assert_eq!(data[0].title, "Space_Adventures_004__c2c__diff_ver");
            #[cfg(target_family = "windows")]
            assert_eq!(
                data[0].path,
                "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver"
            );
            #[cfg(target_family = "unix")]
            assert_eq!(
                data[0].path,
                "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver"
            );

            assert_eq!(data[1].source_id, 1);
            assert_eq!(data[1].number, 1.0_f64);
            assert_eq!(data[1].title, "Space_Adventures_001__c2c__diff_ver");
            #[cfg(target_family = "windows")]
            assert_eq!(
                data[1].path,
                "../../test/data/manga\\Space Adventures\\Space_Adventures_001__c2c__diff_ver.cbz"
            );
            #[cfg(target_family = "unix")]
            assert_eq!(
                data[1].path,
                "../../test/data/manga/Space Adventures/Space_Adventures_001__c2c__diff_ver.cbz"
            );
        }
    }

    #[tokio::test]
    async fn test_archive_get_pages() {
        let local = Local::new("../../test/data/manga");
        #[cfg(target_family = "windows")]
        let pages = local
            .get_pages(
                "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver"
                    .to_string(),
            )
            .await;
        #[cfg(target_family = "unix")]
        let pages = local
            .get_pages(
                "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver"
                    .to_string(),
            )
            .await;

        assert!(pages.is_ok());

        if let Ok(data) = pages {
            assert_eq!(data.len(), 36);

            #[cfg(target_family = "windows")]
            {
                assert_eq!(data[0], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00401.JPG");
                assert_eq!(data[1], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00402.JPG");
                assert_eq!(data[2], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00403.JPG");
                assert_eq!(data[3], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00404.JPG");
                assert_eq!(data[4], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00405.JPG");
                assert_eq!(data[5], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00406.JPG");
                assert_eq!(data[6], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00407.JPG");
                assert_eq!(data[7], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00408.JPG");
                assert_eq!(data[8], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00409.JPG");
                assert_eq!(data[9], "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00410.JPG");
                assert_eq!(
                    data[10],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00411.JPG"
                );
                assert_eq!(
                    data[11],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00412.JPG"
                );
                assert_eq!(
                    data[12],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00413.JPG"
                );
                assert_eq!(
                    data[13],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00414.JPG"
                );
                assert_eq!(
                    data[14],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00415.JPG"
                );
                assert_eq!(
                    data[15],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00416.JPG"
                );
                assert_eq!(
                    data[16],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00417.JPG"
                );
                assert_eq!(
                    data[17],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00418.JPG"
                );
                assert_eq!(
                    data[18],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00419.JPG"
                );
                assert_eq!(
                    data[19],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00420.JPG"
                );
                assert_eq!(
                    data[20],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00421.JPG"
                );
                assert_eq!(
                    data[21],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00422.JPG"
                );
                assert_eq!(
                    data[22],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00423.JPG"
                );
                assert_eq!(
                    data[23],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00424.JPG"
                );
                assert_eq!(
                    data[24],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00425.JPG"
                );
                assert_eq!(
                    data[25],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00426.JPG"
                );
                assert_eq!(
                    data[26],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00427.JPG"
                );
                assert_eq!(
                    data[27],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00428.JPG"
                );
                assert_eq!(
                    data[28],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00429.JPG"
                );
                assert_eq!(
                    data[29],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00430.JPG"
                );
                assert_eq!(
                    data[30],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00431.JPG"
                );
                assert_eq!(
                    data[31],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00432.JPG"
                );
                assert_eq!(
                    data[32],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00433.JPG"
                );
                assert_eq!(
                    data[33],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00434.JPG"
                );
                assert_eq!(
                    data[34],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00435.JPG"
                );
                assert_eq!(
                    data[35],
                    "../../test/data/manga\\Space Adventures\\Space_Adventures_004__c2c__diff_ver\\SPA00436.JPG"
                );
            }

            #[cfg(target_family = "unix")]
            {
                assert_eq!(data[0], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00401.JPG");
                assert_eq!(data[1], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00402.JPG");
                assert_eq!(data[2], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00403.JPG");
                assert_eq!(data[3], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00404.JPG");
                assert_eq!(data[4], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00405.JPG");
                assert_eq!(data[5], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00406.JPG");
                assert_eq!(data[6], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00407.JPG");
                assert_eq!(data[7], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00408.JPG");
                assert_eq!(data[8], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00409.JPG");
                assert_eq!(data[9], "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00410.JPG");
                assert_eq!(
                    data[10],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00411.JPG"
                );
                assert_eq!(
                    data[11],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00412.JPG"
                );
                assert_eq!(
                    data[12],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00413.JPG"
                );
                assert_eq!(
                    data[13],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00414.JPG"
                );
                assert_eq!(
                    data[14],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00415.JPG"
                );
                assert_eq!(
                    data[15],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00416.JPG"
                );
                assert_eq!(
                    data[16],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00417.JPG"
                );
                assert_eq!(
                    data[17],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00418.JPG"
                );
                assert_eq!(
                    data[18],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00419.JPG"
                );
                assert_eq!(
                    data[19],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00420.JPG"
                );
                assert_eq!(
                    data[20],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00421.JPG"
                );
                assert_eq!(
                    data[21],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00422.JPG"
                );
                assert_eq!(
                    data[22],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00423.JPG"
                );
                assert_eq!(
                    data[23],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00424.JPG"
                );
                assert_eq!(
                    data[24],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00425.JPG"
                );
                assert_eq!(
                    data[25],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00426.JPG"
                );
                assert_eq!(
                    data[26],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00427.JPG"
                );
                assert_eq!(
                    data[27],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00428.JPG"
                );
                assert_eq!(
                    data[28],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00429.JPG"
                );
                assert_eq!(
                    data[29],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00430.JPG"
                );
                assert_eq!(
                    data[30],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00431.JPG"
                );
                assert_eq!(
                    data[31],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00432.JPG"
                );
                assert_eq!(
                    data[32],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00433.JPG"
                );
                assert_eq!(
                    data[33],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00434.JPG"
                );
                assert_eq!(
                    data[34],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00435.JPG"
                );
                assert_eq!(
                    data[35],
                    "../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver/SPA00436.JPG"
                );
            }
        }
    }
}
