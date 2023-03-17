use axum::{
    async_trait,
    extract::{FromRequestParts, TypedHeader},
    http::request::Parts,
    RequestPartsExt,
};
use headers::{authorization::Bearer, Authorization};
use serde::Deserialize;

pub struct Token(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for Token
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let token = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map(|TypedHeader(Authorization(bearer))| Token(bearer.token().to_string()))
            .unwrap_or_else(|_| Token("".to_string()));

        Ok(token)
    }
}

pub async fn on_connection_init(
    value: serde_json::Value,
) -> async_graphql::Result<async_graphql::Data> {
    #[derive(Deserialize)]
    struct Payload {
        token: String,
    }

    if let Ok(payload) = serde_json::from_value::<Payload>(value) {
        let mut data = async_graphql::Data::default();
        data.insert(Token(payload.token));
        Ok(data)
    } else {
        Err("Token is required".into())
    }
}
