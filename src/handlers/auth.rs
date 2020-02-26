pub mod auth {
    use crate::auth::{auth::Auth, User};
    use std::convert::Infallible;

    pub async fn register(user: User, auth: Auth) -> Result<impl warp::Reply, Infallible> {
        let res = auth.register(user);
        Ok(warp::reply::json(&res))
    }

    pub async fn login(user: User, auth: Auth) -> Result<impl warp::Reply, Infallible> {
        let res = auth.login(user);
        Ok(warp::reply::json(&res))
    }
}
