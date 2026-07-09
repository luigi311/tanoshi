#[macro_use]
extern crate log;

pub mod myanimelist;
use chrono::NaiveDateTime;
pub use myanimelist::MyAnimeList;

pub mod anilist;
pub use anilist::AniList;

use async_trait::async_trait;
use oauth2::{CsrfToken, PkceCodeVerifier};
use serde::Deserialize;
use thiserror::Error;

/// oauth2 v5 tracks which endpoints are configured in the type. Every tracker
/// configures the auth and token URLs (and nothing else).
pub type OAuthClient = oauth2::basic::BasicClient<
    oauth2::EndpointSet,    // auth url
    oauth2::EndpointNotSet, // device auth url
    oauth2::EndpointNotSet, // introspection url
    oauth2::EndpointNotSet, // revocation url
    oauth2::EndpointSet,    // token url
>;

/// Build the http client used for token exchanges. Redirects are disabled as
/// recommended by oauth2 to prevent SSRF via the token endpoint, and a total
/// timeout keeps a stalled token endpoint from hanging exchanges forever.
fn oauth_http_client() -> Result<oauth2::reqwest::Client, Error> {
    Ok(oauth2::reqwest::ClientBuilder::new()
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow::anyhow!("{e}"))?)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("unauthorized")]
    Unauthorized,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

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

#[derive(Debug, Clone)]
pub struct TrackerManga {
    pub tracker: String,
    pub tracker_manga_id: String,
    pub title: String,
    pub synopsis: String,
    pub cover_url: String,
    pub status: String,
    pub tracker_status: Option<TrackerStatus>,
}

#[derive(Debug, Default, Clone)]
pub struct TrackerStatus {
    pub tracker: String,
    pub tracker_manga_id: Option<String>,
    pub tracker_manga_title: Option<String>,
    pub status: Option<String>,
    pub score: Option<i64>,
    pub num_chapters_read: Option<i64>,
    pub start_date: Option<NaiveDateTime>,
    pub finish_date: Option<NaiveDateTime>,
}

#[async_trait]
pub trait Tracker: Sync + Send {
    fn get_authorize_url(&self) -> Result<Session, Error>;

    async fn exchange_code(
        &self,
        code: String,
        state: Option<String>,
        csrf_state: Option<String>,
        pkce_code_verifier: Option<String>,
    ) -> Result<Token, Error>;

    async fn refresh_token(&self, refresh_token: String) -> Result<Token, Error>;

    async fn search_manga(&self, token: String, search: String)
        -> Result<Vec<TrackerManga>, Error>;
    async fn get_manga_details(
        &self,
        token: String,
        tracker_manga_id: i64,
    ) -> Result<TrackerManga, Error>;

    #[allow(clippy::too_many_arguments)]
    async fn update_tracker_status(
        &self,
        token: String,
        tracker_manga_id: i64,
        status: Option<String>,
        score: Option<i64>,
        progress: Option<i64>,
        started_at: Option<NaiveDateTime>,
        completed_at: Option<NaiveDateTime>,
    ) -> Result<(), Error>;
}
