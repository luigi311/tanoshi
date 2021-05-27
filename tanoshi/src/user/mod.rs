use crate::context::GlobalContext;
use async_graphql::{Context, Object, Result};
use rand::RngCore;

mod user;
pub use user::{Role, User};

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub role: Role,
    pub exp: usize,
}

#[derive(Default)]
pub struct UserRoot;

#[Object]
impl UserRoot {
    pub async fn login(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "username")] username: String,
        #[graphql(desc = "password")] password: String,
    ) -> Result<String> {
        let user = ctx
            .data_unchecked::<GlobalContext>()
            .userdb
            .get_user_by_username(username)
            .await?;

        if !argon2::verify_encoded(&user.password, password.as_bytes())? {
            return Err("Wrong username or password".into());
        }

        let secret = ctx.data_unchecked::<GlobalContext>().secret.clone();
        let token = jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                sub: user.id,
                username: user.username,
                role: user.role,
                exp: 10000000000,
            },
            &EncodingKey::from_secret(secret.as_bytes()),
        )?;

        Ok(token)
    }
}

#[derive(Default)]
pub struct UserMutationRoot;

#[Object]
impl UserMutationRoot {
    pub async fn register(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "username")] username: String,
        #[graphql(desc = "password")] password: String,
    ) -> Result<i64> {
        let is_admin = check_is_admin(&ctx);

        let userdb = &ctx.data_unchecked::<GlobalContext>().userdb;

        let user_count = userdb.get_users_count().await?;

        if !is_admin && user_count > 0 {
            return Err("Forbidden".into());
        };

        let mut salt: [u8; 32] = [0; 32];
        rand::thread_rng().fill_bytes(&mut salt);

        let config = argon2::Config::default();
        let hash = argon2::hash_encoded(password.as_bytes(), &salt, &config).unwrap();

        // If first user, make it admin else make it reader by default
        let role = if user_count == 0 {
            Role::Admin
        } else {
            Role::Reader
        };

        let user = User {
            id: 0,
            username,
            password: hash,
            role,
        };

        let user_id = userdb.insert_user(user).await?;

        Ok(user_id)
    }
}

pub fn get_claims(ctx: &Context<'_>) -> Option<Claims> {
    let token = if let Some(token) = ctx.data_opt::<String>() {
        token
    } else {
        return None;
    };

    let secret = ctx.data_unchecked::<GlobalContext>().secret.clone();
    if let Ok(data) = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Some(data.claims)
    } else {
        None
    }
}

pub fn check_is_admin(ctx: &Context<'_>) -> bool {
    if let Some(claims) = get_claims(ctx) {
        if claims.role == Role::Admin {
            return true;
        }
    }

    return false;
}
