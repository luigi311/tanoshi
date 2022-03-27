use anyhow::Result;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    TokenUrl,
};
use serde::Deserialize;

use super::Session;

pub const NAME: &'static str = "myanimelist";

#[derive(Debug, Clone, Deserialize)]
pub struct MainPicture {
    pub medium: String,
    pub large: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manga {
    pub id: i64,
    pub title: String,
    pub synopsis: String,
    pub main_picture: MainPicture,
    pub status: String,
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
                ("limit", format!("{limit}")),
                ("offset", format!("{offset}")),
            ])
            .send()
            .await?
            .json()
            .await?;
        Ok(res.data.into_iter().map(|node| node.node).collect())
    }
}
