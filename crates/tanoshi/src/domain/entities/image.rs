use aes::cipher::{block_padding::Pkcs7, BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};
use anyhow::anyhow;
use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use fancy_regex::Regex;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use crate::infrastructure::local::SUPPORTED_FILES;

// create an alias for convenience
type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

pub enum ImageUri {
    Remote(String),
    File(String),
    Archive(String, String),
}

impl TryFrom<&str> for ImageUri {
    type Error = anyhow::Error;

    fn try_from(uri: &str) -> Result<Self, Self::Error> {       
        let uri = if uri.starts_with("http") {
            Self::Remote(uri.to_string())
        } else if !uri.is_empty() {
            let path = std::path::PathBuf::from(uri);
            if path.is_file() {
                Self::File(uri.to_string())
            } else {
                let regex = format!(r#"\.({})[\/|\\]"#, SUPPORTED_FILES.iter().join("|"));
                let re = Regex::new(&regex)?;

                if let Ok(Some(matches)) = re.find(uri) {
                    let archive = uri[0..matches.end() - 1].to_owned();
                    let filename = uri[matches.end()..uri.len()].to_owned();

                    Self::Archive(archive, filename)
                } else {
                    return Err(anyhow!("invalid file uri"));
                }
            }
        } else {
            return Err(anyhow!("bad uri"));
        };

        Ok(uri)
    }
}

impl ImageUri {
    pub fn from_encrypted(secret: &str, encrypted: &str) -> Result<Self, anyhow::Error> {
        let mut decoded = general_purpose::URL_SAFE_NO_PAD.decode(encrypted)?;
        trace!("decoded: {decoded:?}");

        let iv = [0_u8; 16];

        let bytes = Aes128CbcDec::new_from_slices(secret.as_bytes(), &iv)
            .map_err(|e| anyhow!("invalid key length: {e}"))?
            .decrypt_padded::<Pkcs7>(&mut decoded)
            .map_err(|e| anyhow::anyhow!("error decrypt url {e}"))?
            .to_vec();

        let url = String::from_utf8(bytes)?;
        let uri = ImageUri::try_from(url.as_str())?;

        Ok(uri)
    }

    pub fn into_encrypted(self, secret: &str) -> Result<String, anyhow::Error> {
        let uri = self.to_string();
        let pos = uri.len();

        let mut buffer = vec![0_u8; pos * 2];
        buffer.splice(..pos, uri.as_bytes().to_vec());

        let iv = [0_u8; 16];
        let chipertext = Aes128CbcEnc::new_from_slices(secret.as_bytes(), &iv)
            .map_err(|e| anyhow!("invalid key length: {e}"))?
            .encrypt_padded::<Pkcs7>(&mut buffer, pos)
            .map_err(|e| anyhow!("error encrypt url {e}"))?;

        let encoded = general_purpose::URL_SAFE_NO_PAD.encode(chipertext);

        Ok(encoded)
    }
}

impl std::fmt::Display for ImageUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageUri::Remote(url) => write!(f, "{url}"),
            ImageUri::File(path) => write!(f, "{path}"),
            ImageUri::Archive(archive, filename) => write!(f, "{archive}/{filename}"),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Image {
    pub content_type: String,
    pub data: Bytes,
}
