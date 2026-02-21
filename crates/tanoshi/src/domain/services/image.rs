use crate::domain::{
    entities::image::{Image, ImageUri},
    repositories::{
        image::{ImageRepository, ImageRepositoryError},
        image_cache::{ImageCacheRepository, ImageCacheRepositoryError},
    },
};
use std::convert::TryFrom;
use thiserror::Error;
use tanoshi_vm::extension::ExtensionManager;

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("error request image")]
    RequestError,
    #[error("repository error: {0}")]
    RepositoryError(#[from] ImageRepositoryError),
    #[error("cache error: {0}")]
    CacheError(#[from] ImageCacheRepositoryError),
    #[error("other error: {0}")]
    Other(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct ImageService<C, R>
where
    C: ImageCacheRepository,
    R: ImageRepository,
{
    repo: R,
    cache_repo: C,
    ext: ExtensionManager,
}

impl<C, R> ImageService<C, R>
where
    C: ImageCacheRepository,
    R: ImageRepository,
{
    pub fn new(repo: R, cache_repo: C, ext: ExtensionManager) -> Self {
        Self { repo, cache_repo, ext }
    }

    pub async fn fetch_image(
        &self,
        secret: &str,
        encrypted_url: &str,
        referer: Option<&String>,
    ) -> Result<Image, ImageError> {
        if let Ok(image) = self.cache_repo.get(encrypted_url).await {
            return Ok(image);
        }

        let uri = ImageUri::from_encrypted(secret, encrypted_url)
            .map_err(|e| ImageError::Other(anyhow::anyhow!("{e}")))?;

        let image = match uri {
            ImageUri::Remote(url) => {
                let image = self.repo.fetch_image_from_url(&url, referer).await?;
                if let Err(e) = self.cache_repo.set(encrypted_url, &image).await {
                    error!("error cache image {encrypted_url}: {e}");
                }

                image
            }
            ImageUri::ExtensionRemote { source_id, url } => {
                let bytes = self
                    .ext
                    .get_image_bytes(source_id, url)
                    .await
                    .map_err(|e| ImageError::Other(anyhow::anyhow!("{e}")))?;

                // You *must* decide a content-type. Either:
                // 1) sniff via header in extension API (best), or
                // 2) sniff by magic bytes, or
                // 3) default "image/jpeg" (least safe)
                let image = Image {
                    content_type: "image/jpeg".to_string(), // TODO: improve
                    data: bytes,
                };

                let _ = self.cache_repo.set(encrypted_url, &image).await;
                image
            }
            ImageUri::File(path) => self.repo.fetch_image_from_file(&path).await?,
            ImageUri::Archive(archive, filename) => {
                self.repo
                    .fetch_image_from_archive(&archive, &filename)
                    .await?
            }
        };

        Ok(image)
    }

    pub fn encrypt_image_url(&self, secret: &str, url: &str) -> Result<String, ImageError> {
        let image_uri = ImageUri::try_from(url)?;

        Ok(image_uri.into_encrypted(secret)?)
    }

    pub fn encrypt_extension_image_url(
        &self,
        secret: &str,
        source_id: i64,
        url: &str,
    ) -> Result<String, ImageError> {
        let image_uri = ImageUri::ExtensionRemote {
            source_id,
            url: url.to_string(),
        };

        Ok(image_uri.into_encrypted(secret)?)
    }
}
