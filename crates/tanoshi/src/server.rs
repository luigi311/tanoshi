use crate::{config::Config, proxy::Proxy, schema::TanoshiSchema};

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
    response::{self, IntoResponse},
    routing::{get, post},
    AddExtensionLayer, Router, Server,
};
use headers::{authorization::Bearer, Authorization};
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};
use tower_http::cors::{any, CorsLayer};

struct Token(String);

#[async_trait]
impl<B> FromRequest<B> for Token
where
    B: Send,
{
    type Rejection = ();

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let token = TypedHeader::<Authorization<Bearer>>::from_request(req)
            .await
            .map(|TypedHeader(Authorization(bearer))| Token(bearer.token().to_string()))
            .unwrap_or_else(|_| Token("".to_string()));

        Ok(token)
    }
}

async fn graphql_handler(
    token: Token,
    schema: Extension<TanoshiSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let req = req.into_inner();
    let req = req.data(token.0);
    schema.execute(req).await.into()
}

#[allow(dead_code)]
async fn graphql_playground() -> impl IntoResponse {
    response::Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

async fn health_check() -> impl IntoResponse {
    response::Html("OK")
}

pub fn init_app(config: &Config, schema: TanoshiSchema) -> Router<axum::body::Body> {
    let proxy = Arc::new(Proxy::new(config.secret.clone()));

    let mut app = Router::new();

    app = app
        .route("/health", get(health_check))
        .route("/image/:url", get(Proxy::proxy))
        .layer(AddExtensionLayer::new(proxy));

    if config.enable_playground {
        app = app
            .route("/graphql", get(graphql_playground).post(graphql_handler))
            .route("/graphql/", post(graphql_handler));
    } else {
        app = app
            .route("/graphql", post(graphql_handler))
            .route("/graphql/", post(graphql_handler));
    }

    app = app.layer(AddExtensionLayer::new(schema)).layer(
        CorsLayer::new()
            .allow_origin(any())
            .allow_methods(any())
            .allow_headers(any())
            .allow_credentials(true),
    );

    #[cfg(feature = "embed")]
    {
        app = app.fallback(get(crate::assets::static_handler));
    }

    app
}

pub async fn serve(
    addr: &str,
    port: u16,
    router: Router<axum::body::Body>,
) -> Result<(), anyhow::Error> {
    let addr = SocketAddr::from((IpAddr::from_str(addr)?, port));
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
