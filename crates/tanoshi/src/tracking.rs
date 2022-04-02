use anyhow::anyhow;
use async_graphql::{Context, Object, Result, SimpleObject};
use oauth2::{reqwest::async_http_client, AuthorizationCode, CsrfToken, PkceCodeVerifier};
use serde::Deserialize;

use crate::{
    db::{model, MangaDatabase, UserDatabase},
    tracker::{self, myanimelist, MyAnimeList},
    user::Claims,
};

#[derive(SimpleObject)]
pub struct Session {
    pub authorize_url: String,
    pub csrf_state: String,
    pub pkce_code_verifier: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Token {
    pub token_type: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

async fn exchange_code(
    code: String,
    state: String,
    csrf_state: String,
    pkce_code_verifier: Option<String>,
    client: &MyAnimeList,
) -> anyhow::Result<Token> {
    let code = AuthorizationCode::new(code);
    let _state = CsrfToken::new(state);

    let _csrf_state = CsrfToken::new(csrf_state);
    let pkce_code_verifier = pkce_code_verifier
        .map(|value| PkceCodeVerifier::new(value.to_owned()))
        .ok_or_else(|| anyhow!("no pkce-code-verifier cookie"))?;

    let token = client
        .oauth_client
        .exchange_code(code)
        .set_pkce_verifier(pkce_code_verifier)
        .request_async(async_http_client)
        .await?;

    let token_str = serde_json::to_string(&token)?;
    Ok(serde_json::from_str(&token_str)?)
}

#[derive(Default, SimpleObject)]
pub struct TrackerManga {
    pub tracker: String,
    pub tracker_manga_id: String,
    pub title: String,
    pub synopsis: String,
    pub cover_url: String,
    pub status: String,
}

#[derive(Default)]
pub struct TrackingRoot;

#[Object]
impl TrackingRoot {
    async fn myanimelist_login_start(&self, ctx: &Context<'_>) -> Result<Session> {
        let _ = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;
        let session = ctx.data::<MyAnimeList>()?.get_authorize_url().unwrap();
        Ok(Session {
            authorize_url: session.authorize_url,
            csrf_state: session.csrf_state.secret().to_owned(),
            pkce_code_verifier: session
                .pkce_code_verifier
                .map(|val| val.secret().to_owned()),
        })
    }

    async fn myanimelist_login_end(
        &self,
        ctx: &Context<'_>,
        code: String,
        state: String,
        csrf_state: String,
        pkce_code_verifier: Option<String>,
    ) -> Result<String> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;
        let client = ctx.data::<MyAnimeList>()?;
        let token = exchange_code(code, state, csrf_state, pkce_code_verifier, client)
            .await
            .map(|token| model::Token {
                token_type: token.token_type,
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                expires_in: token.expires_in,
            })?;
        ctx.data::<UserDatabase>()?
            .insert_tracker_credential(user.sub, tracker::myanimelist::NAME, token)
            .await?;
        Ok("Success".to_string())
    }

    async fn search_tracker_manga(
        &self,
        ctx: &Context<'_>,
        tracker: String,
        title: String,
    ) -> Result<Vec<TrackerManga>> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        match tracker.as_str() {
            myanimelist::NAME => {
                let tracker_token = ctx
                    .data::<UserDatabase>()?
                    .get_user_tracker_token(tracker::myanimelist::NAME, user.sub)
                    .await?;

                let manga_list = ctx
                    .data::<MyAnimeList>()?
                    .get_manga_list(
                        tracker_token.access_token,
                        title,
                        6,
                        0,
                        "id,title,main_picture,synopsis,status".to_string(),
                    )
                    .await?;

                Ok(manga_list
                    .into_iter()
                    .map(|m| TrackerManga {
                        tracker: myanimelist::NAME.to_string(),
                        tracker_manga_id: m.id.to_string(),
                        title: m.title,
                        synopsis: m.synopsis,
                        cover_url: m.main_picture.medium,
                        status: m.status,
                    })
                    .collect())
            }
            _ => Err("tracker not available".into()),
        }
    }
}

#[derive(Default)]
pub struct TrackingMutationRoot;

#[Object]
impl TrackingMutationRoot {
    async fn track_manga(
        &self,
        ctx: &Context<'_>,
        tracker: String,
        manga_id: i64,
        tracker_manga_id: String,
    ) -> Result<i64> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        if !matches!(tracker.as_str(), myanimelist::NAME) {
            return Err("tracker not available".into());
        }

        Ok(ctx
            .data::<MangaDatabase>()?
            .insert_tracker_manga(user.sub, manga_id, &tracker, tracker_manga_id)
            .await?)
    }
}
