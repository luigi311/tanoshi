use warp::{Reply, Filter, filters::BoxedFilter};
use crate::handlers::static_files;

pub fn static_files() -> BoxedFilter<(impl Reply,)> {
    serve().or(serve_index()).boxed()
}

fn serve_index() -> BoxedFilter<(impl Reply,)> {
    warp::get()
    .and(with_encoding())
    .and_then(static_files::serve_index).boxed()
}

fn serve() -> BoxedFilter<(impl Reply,)> {
    warp::get()
        .and(warp::path::tail())
        .and(with_encoding())
        .and_then(static_files::serve_tail).boxed()
}

fn with_encoding() -> impl Filter<Extract = (static_files::Encoding,), Error = warp::reject::Rejection> + Clone {
    warp::header::header("accept-encoding")
    .map(move |encoding: String| if encoding == "" {
        static_files::Encoding::None
    } else if encoding.contains("br"){
        static_files::Encoding::Br
    } else if encoding.contains("gzip") {
        static_files::Encoding::Gzip
    } else {
        static_files::Encoding::None
    })
}