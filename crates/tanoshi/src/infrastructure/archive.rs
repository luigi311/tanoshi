use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use sevenz_rust2::Password;

/// Unified reader for the comic archive formats supported by the local source.
///
/// Backend-specific APIs are intentionally contained in this module so callers
/// only depend on listing entries and reading one entry into memory.
pub struct ArchiveReader {
    path: PathBuf,
    backend: ArchiveBackend,
}

enum ArchiveBackend {
    Zip(zip::ZipArchive<File>),
    SevenZip(sevenz_rust2::ArchiveReader<File>),
    Rar,
}

impl ArchiveReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);

        let backend = match extension.as_deref() {
            Some("cbz") => {
                let file = File::open(&path)
                    .with_context(|| format!("failed to open ZIP archive {}", path.display()))?;
                ArchiveBackend::Zip(
                    zip::ZipArchive::new(file).with_context(|| {
                        format!("failed to read ZIP archive {}", path.display())
                    })?,
                )
            }
            Some("cb7") => ArchiveBackend::SevenZip(
                sevenz_rust2::ArchiveReader::open(&path, Password::empty())
                    .with_context(|| format!("failed to read 7z archive {}", path.display()))?,
            ),
            Some("cbr") => ArchiveBackend::Rar,
            _ => bail!("unsupported archive format: {}", path.display()),
        };

        Ok(Self { path, backend })
    }

    pub fn list_files(&mut self) -> Result<Vec<String>> {
        match &mut self.backend {
            ArchiveBackend::Zip(archive) => (0..archive.len())
                .filter_map(|index| {
                    let result = archive
                        .by_index(index)
                        .with_context(|| {
                            format!(
                                "failed to read entry {index} from ZIP archive {}",
                                self.path.display()
                            )
                        });
                    match result {
                        Ok(file) if file.is_dir() => None,
                        Ok(file) => Some(Ok(file.name().to_owned())),
                        Err(err) => Some(Err(err)),
                    }
                })
                .collect(),
            ArchiveBackend::SevenZip(archive) => Ok(archive
                .archive()
                .files
                .iter()
                .map(|file| file.name().to_owned())
                .collect()),
            ArchiveBackend::Rar => {
                let source = File::open(&self.path).with_context(|| {
                    format!("failed to reopen RAR archive {}", self.path.display())
                })?;
                compress_tools::list_archive_files(source).map_err(Into::into)
            }
        }
    }

    pub fn read_file(&mut self, filename: &str) -> Result<Vec<u8>> {
        match &mut self.backend {
            ArchiveBackend::Zip(archive) => {
                let mut file = archive.by_name(filename).with_context(|| {
                    format!(
                        "failed to find {filename} in ZIP archive {}",
                        self.path.display()
                    )
                })?;
                let capacity = file.size().min(4 << 20).try_into().unwrap_or(0);
                let mut data = Vec::with_capacity(capacity);
                file.read_to_end(&mut data).with_context(|| {
                    format!(
                        "failed to read {filename} from ZIP archive {}",
                        self.path.display()
                    )
                })?;
                Ok(data)
            }
            ArchiveBackend::SevenZip(archive) => archive.read_file(filename).with_context(|| {
                format!(
                    "failed to read {filename} from 7z archive {}",
                    self.path.display()
                )
            }),
            ArchiveBackend::Rar => {
                let source = File::open(&self.path).with_context(|| {
                    format!("failed to reopen RAR archive {}", self.path.display())
                })?;
                let mut data = Vec::new();
                compress_tools::uncompress_archive_file(source, &mut data, filename)
                    .with_context(|| {
                        format!(
                            "failed to read {filename} from RAR archive {}",
                            self.path.display()
                        )
                    })?;
                Ok(data)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cbz_fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/data/manga/Space_Adventures_004__c2c__diff_ver.cbz")
    }

    fn archive_fixture(filename: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/data/archive")
            .join(filename)
    }

    fn page_fixture(filename: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/data/manga/Space Adventures/Space_Adventures_004__c2c__diff_ver")
            .join(filename)
    }

    #[test]
    fn lists_and_reads_cbz_entries() {
        let mut archive = ArchiveReader::open(cbz_fixture()).unwrap();
        let files = archive.list_files().unwrap();

        assert_eq!(files.len(), 36);
        assert!(files.iter().any(|file| file == "SPA00401.JPG"));
        assert_eq!(
            archive.read_file("SPA00401.JPG").unwrap(),
            std::fs::read(page_fixture("SPA00401.JPG")).unwrap()
        );
    }

    #[test]
    fn lists_and_reads_cb7_entries() {
        let mut archive =
            ArchiveReader::open(archive_fixture("Space_Adventures_004__c2c__diff_ver.cb7"))
                .unwrap();
        let files = archive.list_files().unwrap();

        assert_eq!(files.len(), 36);
        assert!(files.iter().any(|file| file == "SPA00401.JPG"));
        assert_eq!(
            archive.read_file("SPA00401.JPG").unwrap(),
            std::fs::read(page_fixture("SPA00401.JPG")).unwrap()
        );
    }

    #[test]
    fn lists_and_reads_cbr_entries() {
        let mut archive =
            ArchiveReader::open(archive_fixture("Space_Adventures_004__c2c__diff_ver.cbr"))
                .unwrap();
        let files = archive.list_files().unwrap();

        assert_eq!(files.len(), 36);
        assert!(files.iter().any(|file| file == "SPA00401.JPG"));
        assert_eq!(
            archive.read_file("SPA00401.JPG").unwrap(),
            std::fs::read(page_fixture("SPA00401.JPG")).unwrap()
        );
    }
}
