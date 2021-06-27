#[macro_use]
extern crate log;
extern crate argon2;

mod assets;
mod catalogue;
mod config;
mod context;
mod db;
mod extension;
mod library;
mod proxy;
mod schema;
mod status;
mod user;
mod local;

use crate::{
    config::Config,
    context::GlobalContext,
    extension::ExtensionBus,
    schema::{MutationRoot, QueryRoot, TanoshiSchema},
};
use clap::Clap;

use async_graphql::{
    extensions::ApolloTracing,
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, Schema,
};
use async_graphql_warp::{BadRequest, Response};
use std::convert::Infallible;
use warp::{
    http::{Response as HttpResponse, StatusCode},
    Filter, Rejection,
};

#[derive(Clap)]
struct Opts {
    /// Path to config file
    #[clap(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let opts: Opts = Opts::parse();
    let config = Config::open(opts.config)?;

    let (_, extension_tx) = extension::start(config.clone());
    extension::load(config.plugin_path.clone(), extension_tx.clone()).await?;

    let pool = db::establish_connection(config.database_path).await;
    let mangadb = db::MangaDatabase::new(pool.clone());
    let userdb = db::UserDatabase::new(pool.clone());

    let extension_bus = ExtensionBus::new(config.plugin_path, extension_tx);
    let schema: TanoshiSchema = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription::default(),
    )
    .extension(ApolloTracing)
    .data(GlobalContext::new(
        userdb,
        mangadb,
        config.secret,
        extension_bus,
    ))
    .finish();

    let graphql_post = warp::header::optional::<String>("Authorization")
        .and(async_graphql_warp::graphql(schema.clone()))
        .and_then(
            |token: Option<String>,
             (schema, mut request): (TanoshiSchema, async_graphql::Request)| async move {
                if let Some(token) = token {
                    if let Some(token) = token.strip_prefix("Bearer ").map(|t| t.to_string()) {
                        request = request.data(token);
                    }
                }
                let resp = schema.execute(request).await;
                Ok::<_, Infallible>(Response::from(resp))
            },
        );

    let graphql_playground = warp::path!("graphql").and(warp::get()).map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
    });

    let static_files = assets::filter::static_files();

    let cors = warp::cors().allow_any_origin().allow_method("POST");
    let routes = proxy::proxy()
        .or(graphql_playground)
        .or(static_files)
        .or(graphql_post)
        .recover(|err: Rejection| async move {
            if let Some(BadRequest(err)) = err.find() {
                return Ok::<_, Infallible>(warp::reply::with_status(
                    err.to_string(),
                    StatusCode::BAD_REQUEST,
                ));
            }

            Ok(warp::reply::with_status(
                "INTERNAL_SERVER_ERROR".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        })
        .with(cors);

    warp::serve(routes).run(([0, 0, 0, 0], config.port)).await;

    return Ok(());
}
