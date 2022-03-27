use anyhow::Result;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    TokenUrl,
};

use super::Session;

#[derive(Debug, Clone)]
pub struct MyAnimeList {
    pub client: BasicClient,
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

        Ok(Self { client })
    }

    pub fn get_authorize_url(&self) -> Result<Session> {
        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_plain();
        let (authorize_url, csrf_state) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_code_challenge)
            .url();

        Ok(Session {
            authorize_url: authorize_url.to_string(),
            csrf_state,
            pkce_code_verifier: Some(pkce_code_verifier),
        })
    }
}
