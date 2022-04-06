use anyhow::Result;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, TokenUrl,
};

use super::{Session, Token};

pub const NAME: &'static str = "anilist";

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
}
