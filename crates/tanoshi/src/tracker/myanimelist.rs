use std::collections::HashMap;

use anyhow::Result;
use chrono::NaiveDateTime;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    TokenUrl,
};
use serde::Deserialize;

use super::Session;

pub const NAME: &'static str = "myanimelist";

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct MyListStatus {
    pub status: Option<String>,
    pub score: i64,
    pub num_chapters_read: i64,
    pub start_date: Option<NaiveDateTime>,
    pub finish_date: Option<NaiveDateTime>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct MainPicture {
    pub medium: String,
    pub large: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Manga {
    pub id: i64,
    pub title: String,
    pub synopsis: String,
    pub main_picture: MainPicture,
    pub status: String,
    pub my_list_status: Option<MyListStatus>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Node<T> {
    pub node: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetMangaListResponse {
    pub data: Vec<Node<Manga>>,
}

#[derive(Debug, Clone)]
pub struct MyAnimeList {
    pub oauth_client: BasicClient,
    api_client: reqwest::Client,
}

impl MyAnimeList {
    pub fn new(base_url: &str, client_id: String, client_secret: String) -> Result<Self> {
        let client_id = ClientId::new(client_id);
        let client_secret = ClientSecret::new(client_secret);
        let authorization_url =
            AuthUrl::new("https://myanimelist.net/v1/oauth2/authorize".to_string())?;
        let token_url = TokenUrl::new("https://myanimelist.net/v1/oauth2/token".to_string())?;

        let redirect_url = RedirectUrl::new(format!("{base_url}/tracker/myanimelist/redirect"))?;
        let client = BasicClient::new(
            client_id,
            Some(client_secret),
            authorization_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        Ok(Self {
            oauth_client: client,
            api_client: reqwest::Client::new(),
        })
    }

    pub fn get_authorize_url(&self) -> Result<Session> {
        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_plain();
        let (authorize_url, csrf_state) = self
            .oauth_client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_code_challenge)
            .url();

        Ok(Session {
            authorize_url: authorize_url.to_string(),
            csrf_state,
            pkce_code_verifier: Some(pkce_code_verifier),
        })
    }

    pub async fn get_manga_list(
        &self,
        token: String,
        q: String,
        limit: i64,
        offset: i64,
        fields: String,
    ) -> Result<Vec<Manga>> {
        let res: GetMangaListResponse = self
            .api_client
            .get("https://api.myanimelist.net/v2/manga")
            .bearer_auth(token)
            .query(&[
                ("q", q),
                ("fields", fields),
                ("nsfw", "true".to_string()),
                ("limit", format!("{limit}")),
                ("offset", format!("{offset}")),
            ])
            .send()
            .await?
            .json()
            .await?;
        Ok(res.data.into_iter().map(|node| node.node).collect())
    }

    pub async fn get_manga_details(
        &self,
        token: String,
        tracker_manga_id: String,
        fields: String,
    ) -> Result<Manga> {
        let res: Manga = self
            .api_client
            .get(format!(
                "https://api.myanimelist.net/v2/manga/{tracker_manga_id}"
            ))
            .bearer_auth(token)
            .query(&[("fields", &fields)])
            .send()
            .await?
            .json()
            .await?;

        Ok(res)
    }

    pub async fn update_my_list_status(
        &self,
        token: String,
        tracker_manga_id: String,
        params: &[(&str, &str)],
    ) -> Result<()> {
        let params: HashMap<&str, &str> = params.iter().map(|param| param.to_owned()).collect();

        self.api_client
            .patch(format!(
                "https://api.myanimelist.net/v2/manga/{tracker_manga_id}/my_list_status"
            ))
            .bearer_auth(token)
            .form(&params)
            .send()
            .await?;

        Ok(())
    }
}
