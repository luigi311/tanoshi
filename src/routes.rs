#[macro_export]
macro_rules! bind_routes {
    ($port:expr, $route:ident, $($routes:ident),+) => {{
        let cors = warp::cors().allow_any_origin().allow_method("POST");
        let routes = $route
        $(.or($routes))+
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

        warp::serve(routes).run(([0, 0, 0, 0], $port as u16)).await;
    }};
}
