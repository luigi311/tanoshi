use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("version has invalid format, should be major.minor.patch")]
    InvalidVersion,
}
