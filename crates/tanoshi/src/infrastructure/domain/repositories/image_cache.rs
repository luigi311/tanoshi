use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::domain::{
    entities::image::Image,
    repositories::image_cache::{ImageCacheRepository, ImageCacheRepositoryError},
};

// Header identifying the postcard cache format. Files without it predate the
// switch from bincode; they are discarded instead of decoded because a bincode
// payload can accidentally parse as valid postcard and yield a corrupt image.
const MAGIC: &[u8; 4] = b"TIC1";

#[derive(Clone)]
pub struct ImageCacheRepositoryImpl {
    path: PathBuf,
}

impl ImageCacheRepositoryImpl {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: PathBuf::new().join(path),
        }
    }
}

#[async_trait]
impl ImageCacheRepository for ImageCacheRepositoryImpl {
    async fn set(&self, key: &str, image: &Image) -> Result<(), ImageCacheRepositoryError> {
        let path = self.path.join(key);

        let payload = postcard::to_allocvec(image)?;
        let mut encoded = Vec::with_capacity(MAGIC.len() + payload.len());
        encoded.extend_from_slice(MAGIC);
        encoded.extend_from_slice(&payload);
        tokio::fs::write(&path, encoded).await?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Image, ImageCacheRepositoryError> {
        let path = self.path.join(key);

        let encoded = tokio::fs::read(&path).await?;

        let Some(payload) = encoded.strip_prefix(MAGIC) else {
            // Old bincode-era entry: remove it so the caller refetches the
            // image and rewrites the cache in the current format.
            let _ = tokio::fs::remove_file(&path).await;
            return Err(ImageCacheRepositoryError::Other(
                "stale image cache entry, discarding".to_string(),
            ));
        };

        let decoded = postcard::from_bytes(payload)?;

        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn temp_cache_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-image-cache-test-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_roundtrip() {
        let dir = temp_cache_dir("roundtrip");
        let repo = ImageCacheRepositoryImpl::new(&dir);

        let image = Image {
            content_type: "image/jpeg".to_string(),
            data: Bytes::from_static(b"not really a jpeg"),
        };

        repo.set("key", &image).await.unwrap();
        let cached = repo.get("key").await.unwrap();

        assert_eq!(cached.content_type, image.content_type);
        assert_eq!(cached.data, image.data);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_legacy_bincode_entry_is_discarded() {
        let dir = temp_cache_dir("legacy");
        let repo = ImageCacheRepositoryImpl::new(&dir);

        // bincode(legacy) encoding of an Image: u64 LE string length prefix.
        // Must not decode successfully, and must be deleted afterwards.
        let mut legacy = Vec::new();
        legacy.extend_from_slice(&10u64.to_le_bytes());
        legacy.extend_from_slice(b"image/jpeg");
        legacy.extend_from_slice(&4u64.to_le_bytes());
        legacy.extend_from_slice(b"data");
        std::fs::write(dir.join("key"), legacy).unwrap();

        assert!(repo.get("key").await.is_err());
        assert!(!dir.join("key").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
