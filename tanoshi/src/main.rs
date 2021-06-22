extern crate pretty_env_logger;
#[macro_use]
extern crate log;
extern crate argon2;

// mod auth;
// mod assets;
// mod catalogue;
mod config;
// mod context;
// mod db;
// mod extension;
// mod library;
// mod proxy;
// mod schema;
// mod status;
// mod user;

use std::path::PathBuf;

use anyhow::Result;
use clap::Clap;
use tanoshi_lib::prelude::Source;
use tokio::sync::mpsc;
use wasmer::{imports, ChainableNamedResolver, Function, Instance, Module, Store, WasmerEnv};
use wasmer_wasi::{Pipe, WasiEnv, WasiState};

use crate::config::Config;

// use crate::context::GlobalContext;
// use crate::schema::{MutationRoot, QueryRoot, TanoshiSchema};
// use async_graphql::extensions::ApolloTracing;
// use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
// use async_graphql::{EmptySubscription, Schema};
// use async_graphql_warp::{BadRequest, Response};
// use config::Config;
// use std::convert::Infallible;
// use warp::http::{Response as HttpResponse, StatusCode};
// use warp::{Filter, Rejection};

#[derive(Clap)]
#[clap(version = "0.22.4")]
struct Opts {
    /// Path to config file
    #[clap(long)]
    config: Option<String>,
}

fn host_http_request() {
    print!("host_http_request");
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let opts: Opts = Opts::parse();
    let config = Config::open(opts.config)?;

    let secret = config.secret;

    let store = Store::default();

    let extension_path = PathBuf::from(
        "C:\\Users\\fadhlika\\Repos\\tanoshi-extensions\\target\\wasm32-wasi\\debug\\mangasee.wasm",
    );

    let wasm_bytes = std::fs::read(extension_path).unwrap();
    let module = Module::new(&store, wasm_bytes).unwrap();

    let input = Pipe::new();
    let output = Pipe::new();
    let mut wasi_env = WasiState::new("tanoshi")
        .stdin(Box::new(input))
        .stdout(Box::new(output))
        .finalize()
        .unwrap();
    let import_object = wasi_env.import_object(&module).unwrap();

    let tanoshi = imports! {
        "tanoshi" => {
            "host_http_request" => Function::new_native(&store, host_http_request)
        },
    };

    let instance = Instance::new(&module, &tanoshi.chain_back(import_object)).unwrap();

    let detail = instance.exports.get_function("detail").unwrap();
    detail.call(&[]).unwrap();

    let object_str = wasi_read(&wasi_env);
    let source: Source = ron::from_str(&object_str).unwrap();

    print!("source: {:?}", source);

    // let mut extensions = extension::Extensions::new(config.plugin_path.clone());
    // if extensions.initialize(config.plugin_config).is_err() {
    //     log::error!("error initialize plugin");
    // }

    // // let serve_static = filters::static_files::static_files();

    // // let routes = api.or(serve_static).with(warp::log("manga"));
    // let pool = db::establish_connection(config.database_path).await;
    // let mangadb = db::MangaDatabase::new(pool.clone());
    // let userdb = db::UserDatabase::new(pool.clone());

    // let schema: TanoshiSchema = Schema::build(
    //     QueryRoot::default(),
    //     MutationRoot::default(),
    //     EmptySubscription::default(),
    // )
    // .extension(ApolloTracing)
    // .data(GlobalContext::new(userdb, mangadb, secret, extensions))
    // .finish();

    // let graphql_post = warp::header::optional::<String>("Authorization")
    //     .and(async_graphql_warp::graphql(schema.clone()))
    //     .and_then(
    //         |token: Option<String>,
    //          (schema, mut request): (TanoshiSchema, async_graphql::Request)| async move {
    //             if let Some(token) = token {
    //                 if let Some(token) = token.strip_prefix("Bearer ").map(|t| t.to_string()) {
    //                     request = request.data(token);
    //                 }
    //             }
    //             let resp = schema.execute(request).await;
    //             Ok::<_, Infallible>(Response::from(resp))
    //         },
    //     );

    // let graphql_playground = warp::path!("graphql").and(warp::get()).map(|| {
    //     HttpResponse::builder()
    //         .header("content-type", "text/html")
    //         .body(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
    // });

    // let static_files = assets::filter::static_files();

    // let cors = warp::cors().allow_any_origin().allow_method("POST");
    // let routes = proxy::proxy()
    //     .or(graphql_playground)
    //     .or(static_files)
    //     .or(graphql_post)
    //     .recover(|err: Rejection| async move {
    //         if let Some(BadRequest(err)) = err.find() {
    //             return Ok::<_, Infallible>(warp::reply::with_status(
    //                 err.to_string(),
    //                 StatusCode::BAD_REQUEST,
    //             ));
    //         }

    //         Ok(warp::reply::with_status(
    //             "INTERNAL_SERVER_ERROR".to_string(),
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //         ))
    //     })
    //     .with(cors);

    // warp::serve(routes).run(([0, 0, 0, 0], config.port)).await;

    return Ok(());
}

fn wasi_read(wasi_env: &WasiEnv) -> String {
    let mut state = wasi_env.state();
    let wasm_stdout = state.fs.stdout_mut().unwrap().as_mut().unwrap();
    let mut buf = String::new();
    wasm_stdout.read_to_string(&mut buf).unwrap();
    buf
}
