use std::path::Path;

use async_trait::async_trait;

use tanoshi_vm::extension::ExtensionManager;

use crate::domain::{
    entities::image::Image,
    repositories::image::{ImageRepository, ImageRepositoryError},
};

#[derive(Clone)]
pub struct ImageRepositoryImpl {
    extension: ExtensionManager,
}

impl ImageRepositoryImpl {
    pub fn new(extension: ExtensionManager) -> Self {
        Self {
            extension: extension,
        }
    }
}

#[async_trait]
impl ImageRepository for ImageRepositoryImpl {
    async fn fetch_image_from_url(
        &self,
        url: &str,
        source_id: i64,
    ) -> Result<Image, ImageRepositoryError> {
        if url.is_empty() {
            return Err(ImageRepositoryError::Other(
                "url cannot be empty".to_string(),
            ));
        }

        // determine content type from the URL
        let content_type = extract_image_type_from_url(&url);

        let bytes = self
            .extension
            .get_image_bytes(source_id, url.to_string())
            .await
            .map_err(|e| ImageRepositoryError::Other(format!("{e}")))?;

        debug!("fetched image from extension source_id={source_id}, url={}, content_type={content_type}, size={} bytes", &url, bytes.len());
        Ok(Image {
            content_type,
            data: bytes,
        })
    }

    async fn fetch_image_from_file<P>(&self, path: P) -> Result<Image, ImageRepositoryError>
    where
        P: AsRef<Path> + std::marker::Send,
    {
        let content_type = mime_guess::from_path(&path)
            .first_or_octet_stream()
            .to_string();
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| ImageRepositoryError::Other(format!("{e}")))?;

        Ok(Image {
            content_type,
            data: data.into(),
        })
    }

    async fn fetch_image_from_archive<P>(
        &self,
        archive: P,
        filename: &str,
    ) -> Result<Image, ImageRepositoryError>
    where
        P: AsRef<Path> + std::marker::Send,
    {
        let filename = filename.to_owned();
        let content_type = mime_guess::from_path(&filename)
            .first_or_octet_stream()
            .to_string();

        let source = std::fs::File::open(archive)
            .map_err(|e| ImageRepositoryError::Other(format!("{e}")))?;
        let (content_type, data) =
            tokio::task::spawn_blocking(move || -> Result<(String, Vec<u8>), anyhow::Error> {
                let mut buf: Vec<u8> = vec![];
                compress_tools::uncompress_archive_file(source, &mut buf, &filename)?;

                Ok((content_type, buf))
            })
            .await
            .map_err(|e| ImageRepositoryError::Other(format!("{e}")))?
            .map_err(|e| ImageRepositoryError::Other(format!("{e}")))?;

        Ok(Image {
            content_type,
            data: data.into(),
        })
    }

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