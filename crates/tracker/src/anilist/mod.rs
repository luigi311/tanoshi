use std::collections::HashMap;

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, TokenUrl,
};
use serde::Deserialize;
use serde_json::json;

use super::{Session, Token};

pub const NAME: &'static str = "anilist";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MediaListStatus {
    Current,
    Planning,
    Completed,
    Dropped,
    Paused,
    Repeating,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CoverImage {
    pub large: Option<String>,
    pub medium: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct MediaTitle {
    pub romaji: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaListEntry {
    pub id: i64,
    pub status: Option<MediaListStatus>,
    pub progress: Option<i64>,
    pub score: Option<i64>,
    pub started_at: Option<NaiveDate>,
    pub completed_at: Option<NaiveDate>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub id: i64,
    pub title: Option<MediaTitle>,
    pub description: Option<String>,
    pub cover_image: Option<CoverImage>,
    pub status: Option<String>,
    pub media_list_entry: Option<MediaListEntry>,
}

pub struct AniList {
    pub oauth_client: BasicClient,
}

impl AniList {
    pub fn new(base_url: &str, client_id: String, client_secret: String) -> Result<Self> {
        let client_id = ClientId::new(client_id);
        let client_secret = ClientSecret::new(client_secret);
        let authorization_url =
            AuthUrl::new("https://anilist.co/api/v2/oauth/authorize".to_string())?;
        let token_url = TokenUrl::new("https://anilist.co/api/v2/oauth/token".to_string())?;

        let redirect_url = RedirectUrl::new(format!("{base_url}/tracker/{NAME}/redirect"))?;
        let client = BasicClient::new(
            client_id,
            Some(client_secret),
            authorization_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        Ok(Self {
            oauth_client: client,
        })
    }

    pub fn get_authorize_url(&self) -> Result<Session> {
        let (authorize_url, csrf_state) =
            self.oauth_client.authorize_url(CsrfToken::new_random).url();

        Ok(Session {
            authorize_url: authorize_url.to_string(),
            csrf_state,
            pkce_code_verifier: None,
        })
    }

    pub async fn exchange_code(&self, code: String) -> Result<Token> {
        let code = AuthorizationCode::new(code);

        let token = self
            .oauth_client
            .exchange_code(code)
            .request_async(async_http_client)
            .await?;

        let token_str = serde_json::to_string(&token)?;
        Ok(serde_json::from_str(&token_str)?)
    }

    pub async fn search_manga(&self, token: String, search: String) -> Result<Media> {
        const QUERY: &str = "
        query SearchManga($search: String!) {
            Media(search: $search, format_in: [MANGA, ONE_SHOT]) {
              id
              title {
                romaji
              }
              description(asHtml: false)
              coverImage {
                large
                medium
              }
              status
            }
          }
        ";

        let res = reqwest::Client::new()
            .post("https://graphql.anilist.co/")
            .bearer_auth(token)
            .json(&json!({
                "query": QUERY,
                "variables": {
                    "search": search
                }
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        debug!("res: {res:?}");

        let res = res
            .get("data")
            .and_then(|data| data.get("Media"))
            .map(|media| media.to_owned())
            .ok_or_else(|| anyhow!("no data"))?;

        let media: Media = serde_json::from_value(res)?;
        Ok(media)
    }

    pub async fn get_manga_details(&self, token: String, tracker_manga_id: i64) -> Result<Media> {
        const QUERY: &str = "
        query GetManga($id: Int!) {
            Media(id: $id) {
              id
              title {
                romaji
              }
              description(asHtml: false)
              coverImage {
                large
                medium
              }
              status
              mediaListEntry {
                id
                status
                progress
                score(format: POINT_10)
                # startedAt {
                #   year
                #   month
                #   day
                # }
                # completedAt {
                #   year
                #   month
                #   day
                # }
              }
            }
          }
        ";

        let res = reqwest::Client::new()
            .post("https://graphql.anilist.co/")
            .bearer_auth(token)
            .json(&json!({
                "query": QUERY,
                "variables": {
                    "id": tracker_manga_id
                }
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        debug!("res: {res:?}");

        let res = res
            .get("data")
            .and_then(|data| data.get("Media"))
            .map(|media| media.to_owned())
            .ok_or_else(|| anyhow!("no data"))?;

        let media: Media = serde_json::from_value(res)?;
        Ok(media)
    }
    pub async fn save_entry(
        &self,
        token: String,
        id: Option<i64>,
        tracker_manga_id: i64,
        status: Option<String>,
        score: Option<i64>,
        progress: Option<i64>,
        started_at: Option<(i64, i64, i64)>,
        completed_at: Option<(i64, i64, i64)>,
    ) -> Result<()> {
        const QUERY: &str = "
        mutation SaveEntry(
            $id: Int, 
            $mediaId: Int!, 
            $status: MediaListStatus, 
            $score: Int, 
            $progress: Int, 
            $startedAt: FuzzyDateInput, 
            $completedAt: FuzzyDateInput) {
            SaveMediaListEntry(
                id: $id, 
                mediaId: $mediaId, 
                status: $status, 
                scoreRaw: $score, 
                progress: $progress, 
                startedAt: $startedAt, 
                completedAt: $completedAt
            ) {
                id
            }
        }
        ";

        let mut variables = HashMap::new();
        variables.insert("mediaId", json!(tracker_manga_id));
        if let Some(id) = id {
            variables.insert("id", json!(id));
        }
        if let Some(status) = status {
            variables.insert("status", json!(status));
        }
        if let Some(score) = score {
            variables.insert("scoreRaw", json!(score));
        }
        if let Some(progress) = progress {
            variables.insert("progress", json!(progress));
        }
        if let Some((year, month, day)) = started_at {
            variables.insert(
                "startedAt",
                json!({
                    "year": year,
                    "month": month,
                    "day": day
                }),
            );
        }
        if let Some((year, month, day)) = completed_at {
            variables.insert(
                "completedAt",
                json!({
                    "year": year,
                    "month": month,
                    "day": day
                }),
            );
        }

        let res = reqwest::Client::new()
            .post("https://graphql.anilist.co/")
            .bearer_auth(token)
            .json(&json!({
                "query": QUERY,
                "variables": variables
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        debug!("res: {res:?}");

        Ok(())
    }
}
