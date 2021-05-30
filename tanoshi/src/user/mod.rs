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
    async fn login(
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

        let secret = ctx.data::<GlobalContext>()?.secret.clone();
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

    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>> {
        let is_admin = check_is_admin(&ctx);
        if !is_admin {
            return Err("Forbidden".into());
        };

        let users = ctx.data::<GlobalContext>()?.userdb.get_users().await?;

        Ok(users)
    }

    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        let user = get_claims(ctx).ok_or("no token")?;
        let user = ctx
            .data::<GlobalContext>()?
            .userdb
            .get_user_by_id(user.sub)
            .await?;

        Ok(user)
    }
}

#[derive(Default)]
pub struct UserMutationRoot;

#[Object]
impl UserMutationRoot {
    async fn register(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "username")] username: String,
        #[graphql(desc = "password")] password: String,
        #[graphql(desc = "role")] role: Role,
    ) -> Result<i64> {
        let is_admin = check_is_admin(&ctx);

        let userdb = &ctx.data::<GlobalContext>()?.userdb;

        let user_count = userdb.get_users_count().await?;

        if !is_admin && user_count > 0 {
            return Err("Forbidden".into());
        };

        if password.len() < 8 {
            return Err("Password less than 8 character".into());
        }

        let mut salt: [u8; 32] = [0; 32];
        rand::thread_rng().fill_bytes(&mut salt);

        let config = argon2::Config::default();
        let hash = argon2::hash_encoded(password.as_bytes(), &salt, &config).unwrap();

        // If first user, make it admin else make it reader by default
        let role = if user_count == 0 { Role::Admin } else { role };

        let user = User {
            id: 0,
            username,
            password: hash,
            role,
        };

        let user_id = userdb.insert_user(user).await?;

        Ok(user_id)
    }

    async fn change_password(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "old password")] old_password: String,
        #[graphql(desc = "new password")] new_password: String,
    ) -> Result<u64> {
        let claims = get_claims(ctx).ok_or("no_token")?;

        let userdb = &ctx.data::<GlobalContext>()?.userdb;

        let user = userdb.get_user_by_id(claims.sub).await?;

        if !argon2::verify_encoded(&user.password, old_password.as_bytes())? {
            return Err("Wrong old password".into());
        }

        if new_password.len() < 8 {
            return Err("Password less than 8 character".into());
        }

        let mut salt: [u8; 32] = [0; 32];
        rand::thread_rng().fill_bytes(&mut salt);

        let config = argon2::Config::default();
        let hash = argon2::hash_encoded(new_password.as_bytes(), &salt, &config).unwrap();

        let affected = userdb.update_password(user.id, hash).await?;

        Ok(affected)
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
