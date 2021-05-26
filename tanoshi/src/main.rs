extern crate libloading as lib;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

// mod auth;
mod config;
mod extension;
mod proxy;
mod catalogue;
mod context;
mod db;
mod schema;
mod library;

use anyhow::Result;
use clap::Clap;

use crate::context::GlobalContext;
use crate::schema::{QueryRoot, MutationRoot};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_warp::{BadRequest, Response};
use async_graphql::extensions::ApolloTracing;
use config::Config;
use std::convert::Infallible;
use warp::http::{Response as HttpResponse, StatusCode};
use warp::{Filter, Rejection};

#[derive(Clap)]
#[clap(version = "0.14.0")]
struct Opts {
    /// Path to config file
    #[clap(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let opts: Opts = Opts::parse();
    let config = Config::open(opts.config)?;

    let secret = config.secret;
    let mut extensions = extension::Extensions::new();
    if extensions
        .initialize(config.plugin_path.clone(), config.plugin_config)
        .is_err()
    {
        log::error!("error initialize plugin");
    }

    // let serve_static = filters::static_files::static_files();

    // let routes = api.or(serve_static).with(warp::log("manga"));
    let pool = db::establish_connection(config.database_path).await;
    let mangadb = db::MangaDatabase::new(pool);

    let schema = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription::default(),
    )
    //.extension(ApolloTracing)
    .data(GlobalContext::new(mangadb, extensions))
    .finish();

    let graphql_post = async_graphql_warp::graphql(schema).and_then(
        |(schema, request): (
            Schema<QueryRoot, MutationRoot, EmptySubscription>,
            async_graphql::Request,
        )| async move { Ok::<_, Infallible>(Response::from(schema.execute(request).await)) },
    );

    let graphql_playground = warp::path!("graphql").and(warp::get()).map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
    });

    let cors = warp::cors().allow_any_origin().allow_method("POST");
    let routes = proxy::proxy()
        .or(graphql_playground)
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
        }).with(cors);

    warp::serve(routes).run(([0, 0, 0, 0], config.port)).await;

    return Ok(());
}
