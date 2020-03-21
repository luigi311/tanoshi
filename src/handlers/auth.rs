pub mod auth {
    use crate::auth::{auth::Auth, Claims, User};
    use sled::Tree;
    use std::convert::Infallible;

    pub async fn register(
        user: User,
        auth: Auth,
        db: Tree,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = auth.register(user, db);
        Ok(warp::reply::with_status(
            warp::reply::json(&res),
            warp::http::StatusCode::CREATED,
        ))
    }

    pub async fn login(user: User, auth: Auth, db: Tree) -> Result<impl warp::Reply, Infallible> {
        let res = auth.login(user, db);
        Ok(warp::reply::json(&res))
    }

    pub fn validate(token: String, auth: Auth) -> Option<Claims> {
        auth.validate(token)
    }
}
