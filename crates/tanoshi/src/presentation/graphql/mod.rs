pub mod catalogue;
pub mod categories;
pub mod chapter;
pub mod common;
pub mod downloads;
pub mod guard;
pub mod library;
pub mod loader;
pub mod manga;
pub mod notification;
pub mod recent;
pub mod schema;
pub mod source;
pub mod status;
pub mod tracking;
pub mod user;

use crate::infrastructure::{auth, config::Config};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig, ALL_WEBSOCKET_PROTOCOLS},
    Data, Result,
};
use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::{
    extract::{Extension, WebSocketUpgrade},
    response::{self, IntoResponse, Response},
};
use serde::Deserialize;

use self::schema::TanoshiSchema;

use super::token::Token;

pub async fn graphql_handler(
    token: Token,
    Extension(config): Extension<Config>,
    Extension(schema): Extension<TanoshiSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut req = req.into_inner();

    if let Ok(claims) = auth::decode_jwt(&config.secret, &token.0) {
        req = req.data(claims);
    }

    schema.execute(req).await.into()
}

pub async fn graphql_playground() -> impl IntoResponse {
    response::Html(playground_source(
        GraphQLPlaygroundConfig::new("/graphql").subscription_endpoint("/ws"),
    ))
}

async fn on_connection_init(secret: String, value: serde_json::Value) -> Result<Data> {
    #[derive(Debug, Deserialize)]
    struct Payload {
        #[serde(rename = "Authorization")]
        token: String,
    }

    if let Ok(payload) = serde_json::from_value::<Payload>(value) {
        info!("token: {:?}", payload.token);
        let token = payload.token.strip_prefix("Bearer ").unwrap();
        info!("token: {token:?}");

        let mut data = Data::default();
        match auth::decode_jwt(&secret, token) {
            Ok(claims) => {
                info!("claims: {claims:?}");
                data.insert(claims);
            }
            Err(e) => {
                return Err(format!("error: {e:?}").into());
            }
        }
        Ok(data)
    } else {
        Err("Token is required".into())
    }
}

pub async fn graphql_ws_handler(
    Extension(schema): Extension<TanoshiSchema>,
    Extension(config): Extension<Config>,
    protocol: GraphQLProtocol,
    websocket: WebSocketUpgrade,
    token: Token,
) -> Response {
    let mut data = Data::default();
    if let Ok(claims) = auth::decode_jwt(&config.secret, &token.0) {
        data.insert(claims);
    }

    websocket
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |stream| {
            GraphQLWebSocket::new(stream, schema.clone(), protocol)
                .with_data(data)
                .on_connection_init(move |value| on_connection_init(config.secret.clone(), value))
                .serve()
        })
}
