pub mod auth {
    use crate::auth::{auth::Auth, Claims, User};
    use std::convert::Infallible;
    use warp::Filter;

    pub async fn register(user: User, auth: Auth) -> Result<impl warp::Reply, Infallible> {
        let res = auth.register(user);
        Ok(warp::reply::with_status(
            warp::reply::json(&res),
            warp::http::StatusCode::CREATED,
        ))
    }

    pub async fn login(user: User, auth: Auth) -> Result<impl warp::Reply, Infallible> {
        let res = auth.login(user);
        Ok(warp::reply::json(&res))
    }

    pub fn validate() -> impl Filter<Extract = (Claims,), Error = warp::Rejection> + Clone {
        warp::header::header("authorization").and_then(|token: String| async move {
            if let Some(claim) = Auth::validate(token) {
                Ok(claim)
            } else {
                Err(warp::reject())
            }
        })
    }
}
