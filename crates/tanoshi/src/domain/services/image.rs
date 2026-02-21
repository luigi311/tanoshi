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
                // determine content type from the URL
                let content_type = Self::extract_image_type_from_url(&url);

                let bytes = self
                    .ext
                    .get_image_bytes(source_id, url.clone())
                    .await
                    .map_err(|e| ImageError::Other(anyhow::anyhow!("{e}")))?;

                debug!("fetched image from extension source_id={source_id}, url={}, content_type={content_type}, size={} bytes", &url, bytes.len());
                let image = Image {
                    content_type,
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

    fn extract_image_type_from_url(url: &str) -> String {
        let extension = url.split('.').last();

        match extension {
            Some(ext) => match ext.to_lowercase().as_str() {
                "jpg" | "jpeg" => "image/jpeg".to_string(),
                "png" => "image/png".to_string(),
                "gif" => "image/gif".to_string(),
                "bmp" => "image/bmp".to_string(),
                "webp" => "image/webp".to_string(),
                _ => "application/octet-stream".to_string(),
            },
            None => "application/octet-stream".to_string(),
        }
    }
}
