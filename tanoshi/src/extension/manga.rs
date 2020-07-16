use std::io::Read;
use std::sync::{Arc, RwLock};

use serde_json::json;
use warp::Rejection;

use tanoshi_lib::extensions::Extension;
use tanoshi_lib::manga::{GetParams, Params, Source, SourceLogin};
use tanoshi_lib::rest::GetPagesResponse;

use crate::auth::Claims;
use crate::extension::{repository::Repository, Extensions};
use crate::handlers::TransactionReject;

#[derive(Clone)]
pub struct Manga {
    repo: Repository,
    exts: Arc<RwLock<Extensions>>,
}

impl Manga {
    pub fn new(database_path: String, exts: Arc<RwLock<Extensions>>) -> Self {
        Self {
            repo: Repository::new(database_path),
            exts,
        }
    }

    pub async fn list_sources(&self, param: String) -> Result<impl warp::Reply, Rejection> {
        match param.as_str() {
            "available" => {
                let resp = ureq::get(
                    format!("https://raw.githubusercontent.com/faldez/tanoshi-extensions/repo-{}/index.json", std::env::consts::OS).as_str(),
                )
                    .call();
                let sources = resp.into_json_deserialize::<Vec<Source>>().unwrap();
                Ok(warp::reply::json(&json!(
                    {
                        "sources": sources,
                        "status": "success"
                    }
                )))
            }
            "installed" => {
                let exts = self.exts.read().unwrap();
                let sources = exts
                    .extensions()
                    .iter()
                    .map(|(_key, ext)| ext.info())
                    .collect::<Vec<Source>>();

                Ok(warp::reply::json(&json!(
                    {
                        "sources": sources,
                        "status": "success"
                    }
                )))
            }
            _ => Err(warp::reject()),
        }
    }

    pub async fn install_source(
        &self,
        source_name: String,
        plugin_path: String,
    ) -> Result<impl warp::Reply, Rejection> {
        let ext = if cfg!(target_os = "windows") {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "linux") {
            "so"
        } else {
            return Err(warp::reject::custom(TransactionReject {
                message: "os not supported".to_string(),
            }));
        };

        let name = if cfg!(target_os = "windows") {
            source_name.clone()
        } else {
            format!("lib{}", &source_name)
        };

        let file_name = format!("{}.{}", &name, &ext);
        let path = std::path::PathBuf::from(&plugin_path).join(&file_name);

        {
            let mut exts = self.exts.write().unwrap();
            if exts.remove(&source_name).is_ok() {
                if std::fs::remove_file(&path).is_ok() {}
            }
        }

        let resp = ureq::get(
            format!(
                "https://raw.githubusercontent.com/faldez/tanoshi-extensions/repo-{}/library/{}.{}",
                std::env::consts::OS,
                &name,
                &ext,
            )
            .as_str(),
        )
        .call();
        let mut reader = resp.into_reader();
        let mut bytes = vec![];
        if let Err(e) = reader.read_to_end(&mut bytes) {
            return Err(warp::reject::custom(TransactionReject {
                message: e.to_string(),
            }));
        }

        if let Err(e) = std::fs::write(&path, &bytes) {
            return Err(warp::reject::custom(TransactionReject {
                message: e.to_string(),
            }));
        }

        {
            let mut ext = self.exts.write().unwrap();
            if ext.get(&source_name).is_none() {
                unsafe {
                    if let Err(e) = ext.load(path.to_str().unwrap().to_string(), None) {
                        return Err(warp::reject::custom(TransactionReject {
                            message: e.to_string(),
                        }));
                    }
                }
            }
        }

        Ok(warp::reply())
    }

    pub async fn list_mangas(
        &self,
        source: String,
        claim: Claims,
        source_auth: String,
        param: Params,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = self.exts.read().unwrap();
        let mangas = exts
            .get(&source)
            .unwrap()
            .get_mangas(param, source_auth)
            .unwrap();
        debug!("mangas {:?}", mangas.clone());

        let manga_ids = match self.repo.insert_mangas(&source, mangas.clone()) {
            Ok(ids) => {
                debug!("manga ids {:?}", ids);
                ids
            }
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }));
            }
        };
        match self.repo.get_mangas(claim.sub, manga_ids) {
            Ok(mangas) => return Ok(warp::reply::json(&mangas)),
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }));
            }
        }
    }

    pub async fn get_manga_info(
        &self,
        manga_id: i32,
        claim: Claims,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = self.exts.read().unwrap();
        if let Ok(manga) = self.repo.get_manga_detail(manga_id, claim.sub.clone()) {
            if manga.manga.status.is_some()
                && !manga.manga.author.is_empty()
                && manga.manga.description.is_some()
            {
                return Ok(warp::reply::json(&manga));
            }

            let manga = exts
                .get(&manga.manga.source)
                .unwrap()
                .get_manga_info(&manga.manga.path)
                .unwrap();

            match self.repo.update_manga_info(manga_id, manga) {
                Ok(_) => {}
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            }

            match self.repo.get_manga_detail(manga_id, claim.sub) {
                Ok(res) => return Ok(warp::reply::json(&res)),
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            }
        }
        Err(warp::reject())
    }

    pub async fn get_chapters(
        &self,
        manga_id: i32,
        claim: Claims,
        param: GetParams,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = self.exts.read().unwrap();
        let refresh = param.refresh.unwrap_or(false);
        if !refresh {
            match self.repo.get_chapters(manga_id, claim.sub.clone()) {
                Ok(chapter) => return Ok(warp::reply::json(&chapter)),
                Err(_e) => {}
            };
        }

        if let Ok(manga) = self.repo.get_manga(manga_id) {
            let chapter = match exts.get(&manga.source).unwrap().get_chapters(&manga.path) {
                Ok(ch) => ch,
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            };

            match self
                .repo
                .insert_chapters(claim.sub.clone(), manga_id, chapter.clone())
            {
                Ok(_) => {}
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            }

            match self.repo.get_chapters(manga_id, claim.sub) {
                Ok(chapter) => return Ok(warp::reply::json(&chapter)),
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            };
        }
        Err(warp::reject())
    }

    pub async fn get_pages(
        &self,
        chapter_id: i32,
        param: GetParams,
    ) -> anyhow::Result<GetPagesResponse> {
        let exts = self.exts.read().unwrap();

        let refresh = param.refresh.unwrap_or(false);
        if refresh {
            if let Err(e) = self.repo.delete_pages(chapter_id) {
                error!("error delete page: {}", e);
            }
        }

        if let Ok(pages) = self.repo.get_pages(chapter_id) {
            return Ok(pages);
        };

        if let Ok(chapter) = self.repo.get_chapter(chapter_id) {
            let pages = exts
                .get(&chapter.source)
                .unwrap()
                .get_pages(&chapter.path)
                .unwrap();

            match self
                .repo
                .insert_pages(chapter.source.clone(), chapter_id, pages.clone())
            {
                Ok(_) => {}
                Err(e) => {
                    return Err(anyhow::anyhow!("{}", e.to_string()));
                }
            }

            match self.repo.get_pages(chapter_id) {
                Ok(pages) => return Ok(pages),
                Err(e) => {
                    return Err(anyhow::anyhow!("{}", e.to_string()));
                }
            };
        }
        Err(anyhow::anyhow!("pages not found"))
    }

    pub async fn proxy_image(&self, page_id: i32) -> Result<impl warp::Reply, Rejection> {
        let (source, image_url) = match self.repo.get_image_from_page_id(page_id) {
            Ok((source, url)) => (source, url),
            Err(_) => return Err(warp::reject()),
        };

        let exts = self.exts.read().unwrap();
        let bytes = exts.get(&source).unwrap().get_page(&image_url).unwrap();

        let mime = match url::Url::parse(&image_url) {
            Ok(url) => mime_guess::from_path(url.path()).first_or_octet_stream(),
            Err(_) => mime_guess::from_path(&image_url).first_or_octet_stream(),
        };
        let resp = warp::http::Response::builder()
            .header("Content-Type", mime.as_ref())
            .header("Content-Length", bytes.len())
            .body(bytes)
            .unwrap();

        Ok(resp)
    }

    pub async fn source_login(
        &self,
        source: String,
        login_info: SourceLogin,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = self.exts.read().unwrap();
        if let Ok(result) = exts.get(&source).unwrap().login(login_info) {
            return Ok(warp::reply::json(&result));
        }

        Err(warp::reject())
    }
}
