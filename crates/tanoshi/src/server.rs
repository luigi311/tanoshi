use crate::{
    catalogue::{
        chapter::{MangaLoader, NextChapterLoader, PrevChapterLoader, ReadProgressLoader},
        manga::{FavoriteLoader, UserLastReadLoader, UserUnreadChaptersLoader},
    },
    config::Config,
    db::{MangaDatabase, UserDatabase},
    notifier::pushover::Pushover,
    proxy::Proxy,
    schema::{MutationRoot, QueryRoot, TanoshiSchema},
    worker::downloads::DownloadSender,
};
use tanoshi_vm::bus::ExtensionBus;

use async_graphql::{
    dataloader::DataLoader,
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, Schema,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{async_trait, handler::get};
use axum::{
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
    routing::BoxRoute,
};
use axum::{
    handler::post,
    response::{self, IntoResponse},
};
use axum::{AddExtensionLayer, Router, Server};
use headers::{authorization::Bearer, Authorization};
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};
use teloxide::{
    adaptors::{AutoSend, DefaultParseMode},
    Bot,
};

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

fn init_app(
    userdb: UserDatabase,
    mangadb: MangaDatabase,
    config: &Config,
    extension_bus: ExtensionBus,
    download_tx: DownloadSender,
    telegram_bot: Option<DefaultParseMode<AutoSend<Bot>>>,
    pushover: Option<Pushover>,
) -> Router<BoxRoute> {
    let mut schemabuilder = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription::default(),
    )
    // .extension(ApolloTracing)
    .data(DataLoader::new(FavoriteLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(UserLastReadLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(UserUnreadChaptersLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(ReadProgressLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(PrevChapterLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(NextChapterLoader {
        mangadb: mangadb.clone(),
    }))
    .data(DataLoader::new(MangaLoader {
        mangadb: mangadb.clone(),
    }))
    .data(userdb)
    .data(mangadb)
    .data(extension_bus)
    .data(download_tx);

    if let Some(telegram_bot) = telegram_bot {
        schemabuilder = schemabuilder.data(telegram_bot);
    }

    if let Some(pushover) = pushover {
        schemabuilder = schemabuilder.data(pushover);
    }

    let schema: TanoshiSchema = schemabuilder.finish();

    let proxy = Proxy::new(config.secret.clone());

    let mut app = Router::new().boxed();

    #[cfg(feature = "embed")]
    {
        app = app.nest("/", get(crate::assets::static_handler)).boxed();
    }

    app = app
        .route("/image/:url", get(Proxy::proxy))
        .route("/health", get(health_check))
        .layer(AddExtensionLayer::new(proxy))
        .boxed();
    if config.enable_playground {
        app = app
            .nest("/graphql", get(graphql_playground).post(graphql_handler))
            .layer(AddExtensionLayer::new(schema))
            .boxed();
    } else {
        app = app
            .nest("/graphql", post(graphql_handler))
            .layer(AddExtensionLayer::new(schema))
            .boxed();
    }

    app
}

pub async fn serve<T>(
    userdb: UserDatabase,
    mangadb: MangaDatabase,
    config: &Config,
    extension_bus: ExtensionBus,
    download_tx: DownloadSender,
    telegram_bot: Option<DefaultParseMode<AutoSend<Bot>>>,
    pushover: Option<Pushover>,
) -> Result<(), anyhow::Error> {
    let app = init_app(
        userdb,
        mangadb,
        config,
        extension_bus,
        download_tx,
        telegram_bot,
        pushover,
    );

    let addr = SocketAddr::from((IpAddr::from_str("0.0.0.0")?, config.port));
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}
