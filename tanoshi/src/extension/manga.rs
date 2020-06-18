use std::sync::Arc;

use serde_json::json;

use tanoshi_lib::extensions::Extension;
use tanoshi_lib::manga::{GetParams, Params, Source, SourceLogin};
use tokio::sync::RwLock;
use warp::{http::Response, Rejection};

use std::io::Read;

use crate::auth::Claims;
use crate::extension::{repository::Repository, Extensions};
use crate::handlers::TransactionReject;

#[derive(Debug, Clone)]
pub struct Manga {
    repo: Repository,
}

impl Manga {
    pub fn new(database_path: String) -> Self {
        Self {
            repo: Repository::new(database_path),
        }
    }

    pub async fn list_sources(
        &self,
        param: String,
        exts: Arc<RwLock<Extensions>>,
    ) -> Result<impl warp::Reply, Rejection> {
        match param.as_str() {
            "available" => {
                let resp = ureq::get(
                    "https://raw.githubusercontent.com/faldez/tanoshi-extensions/repo/index.json",
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
                let exts = exts.read().await;
                let sources = exts
                    .extensions()
                    .iter()
                    .map(|(key, ext)| {
                        info!("source name {}", key.clone());
                        ext.info()
                    })
                    .collect::<Vec<Source>>();

                match self.repo.insert_sources(sources).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("error get source {}", e.to_string());
                        return Err(warp::reject());
                    }
                }

                match self.repo.get_sources().await {
                    Ok(sources) => {
                        debug!("sources {:?}", sources.clone());
                        Ok(warp::reply::json(&json!(
                            {
                                "sources": sources,
                                "status": "success"
                            }
                        )))
                    }
                    Err(e) => {
                        error!("error get source {}", e.to_string());
                        Err(warp::reject::custom(TransactionReject {
                            message: e.to_string(),
                        }))
                    }
                }
            }
            _ => Err(warp::reject()),
        }
    }

    pub async fn install_source(
        &self,
        name: String,
        exts: Arc<RwLock<Extensions>>,
        plugin_path: String,
    ) -> Result<impl warp::Reply, Rejection> {
        let resp = ureq::get(
            format!(
                "https://raw.githubusercontent.com/faldez/tanoshi-extensions/repo/library/lib{}.so",
                name.clone()
            )
            .as_str(),
        )
        .call();
        let mut reader = resp.into_reader();
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes);

        let path = std::path::PathBuf::from(plugin_path);
        let path = path.join(format!("lib{}.so", name));
        match std::fs::write(path.clone(), &bytes) {
            Ok(_) => Ok(warp::reply()),
            Err(e) => {
                return Err(warp::reject::custom(TransactionReject {
                    message: e.to_string(),
                }))
            }
        }
    }

    pub async fn list_mangas(
        &self,
        source_id: i32,
        claim: Claims,
        source_auth: String,
        param: Params,
        exts: Arc<RwLock<Extensions>>,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = exts.read().await;
        if let Ok(source) = self.repo.get_source(source_id).await {
            let mangas = exts
                .get(&source.name)
                .unwrap()
                .get_mangas(&source.url, param, source_auth)
                .unwrap();
            debug!("mangas {:?}", mangas.clone());

            let manga_ids = match self.repo.insert_mangas(source_id, mangas.clone()).await {
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
            match self.repo.get_mangas(claim.sub, manga_ids).await {
                Ok(mangas) => return Ok(warp::reply::json(&mangas)),
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            }
        }
        Err(warp::reject())
    }

    pub async fn get_manga_info(
        &self,
        manga_id: i32,
        claim: Claims,
        exts: Arc<RwLock<Extensions>>,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = exts.read().await;
        if let Ok(manga) = self
            .repo
            .get_manga_detail(manga_id, claim.sub.clone())
            .await
        {
            return Ok(warp::reply::json(&manga));
        } else if let Ok(url) = self.repo.get_manga_url(manga_id).await {
            let source = match self.repo.get_source_from_manga_id(manga_id).await {
                Ok(source) => source,
                Err(e) => {
                    error!("error get_manga_url {}", e.to_string());
                    return Err(warp::reject());
                }
            };

            let manga = exts
                .get(&source.name)
                .unwrap()
                .get_manga_info(&url)
                .unwrap();

            match self.repo.update_manga_info(manga_id, manga).await {
                Ok(_) => {}
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            }
            match self.repo.get_manga_detail(manga_id, claim.sub).await {
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
        exts: Arc<RwLock<Extensions>>,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = exts.read().await;
        if !param.refresh.unwrap_or(false) {
            match self.repo.get_chapters(manga_id, claim.sub.clone()).await {
                Ok(chapter) => return Ok(warp::reply::json(&chapter)),
                Err(_e) => {}
            };
        }

        if let Ok(url) = self.repo.get_manga_url(manga_id).await {
            let source = match self.repo.get_source_from_manga_id(manga_id).await {
                Ok(source) => source,
                Err(e) => return Err(warp::reject()),
            };

            let chapter = exts.get(&source.name).unwrap().get_chapters(&url).unwrap();

            match self.repo.insert_chapters(manga_id, chapter.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            }

            match self.repo.get_chapters(manga_id, claim.sub).await {
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
        _param: GetParams,
        exts: Arc<RwLock<Extensions>>,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = exts.read().await;
        match self.repo.get_pages(chapter_id).await {
            Ok(pages) => return Ok(warp::reply::json(&pages)),
            Err(_) => {}
        };

        if let Ok(url) = self.repo.get_chapter_url(chapter_id).await {
            let source = match self.repo.get_source_from_chapter_id(chapter_id).await {
                Ok(source) => source,
                Err(e) => return Err(warp::reject()),
            };

            let pages = exts.get(&source.name).unwrap().get_pages(&url).unwrap();

            match self.repo.insert_pages(chapter_id, pages.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            }

            match self.repo.get_pages(chapter_id).await {
                Ok(pages) => return Ok(warp::reply::json(&pages)),
                Err(e) => {
                    return Err(warp::reject::custom(TransactionReject {
                        message: e.to_string(),
                    }));
                }
            };
        }
        Err(warp::reject())
    }

    pub async fn proxy_image(
        &self,
        page_id: i32,
        exts: Arc<RwLock<Extensions>>,
    ) -> Result<impl warp::Reply, Rejection> {
        let image = match self.repo.get_image_from_page_id(page_id).await {
            Ok(image) => image,
            Err(_) => return Err(warp::reject()),
        };

        let exts = exts.read().await;
        let bytes = exts
            .get(&image.source_name)
            .unwrap()
            .get_page(image.clone())
            .unwrap();

        let path = std::path::PathBuf::from(image.path).join(image.file_name);
        let mime = mime_guess::from_path(&path).first_or_octet_stream();
        let resp = Response::builder()
            .header("Content-Type", mime.as_ref())
            .header("Content-Length", bytes.len())
            .body(bytes)
            .unwrap();

        Ok(resp)
    }

    pub async fn source_login(
        &self,
        source_id: i32,
        login_info: SourceLogin,
        exts: Arc<RwLock<Extensions>>,
    ) -> Result<impl warp::Reply, Rejection> {
        let exts = exts.read().await;
        if let Ok(source) = self.repo.get_source(source_id).await {
            if let Ok(result) = exts.get(&source.name).unwrap().login(login_info) {
                let mut result = result;
                result.source_id = source_id;
                return Ok(warp::reply::json(&result));
            }
        }
        Err(warp::reject())
    }
}
