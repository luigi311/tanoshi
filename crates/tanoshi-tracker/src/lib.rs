#[macro_use]
extern crate log;

pub mod myanimelist;
pub use myanimelist::MyAnimeList;

pub mod anilist;
pub use anilist::AniList;

use oauth2::{CsrfToken, PkceCodeVerifier};
use serde::Deserialize;

#[derive(Debug)]
pub struct Session {
    pub authorize_url: String,
    pub csrf_state: CsrfToken,
    pub pkce_code_verifier: Option<PkceCodeVerifier>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    pub token_type: String,
    pub expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
}
