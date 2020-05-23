use crate::auth::{Claims, User, UserResponse};
use argon2::{self, Config};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand;
use rand::Rng;
use sqlx::{self, Row};
use sqlx::postgres::{PgPool, PgRow};
use anyhow::Result;

#[derive(Clone)]
pub struct Auth {}

impl Auth {
    pub async fn register(user: User, db: PgPool) -> UserResponse {
        let hashed = Auth::hash(user.password.unwrap_or("tanoshi123".to_string()).as_bytes());
        match sqlx::query!(
            r#"INSERT INTO "user"(username, password, role) VALUES ($1, $2, $3)"#,
            user.username,
            hashed,
            user.role,
        )
        .execute(&db)
        .await
        {
            Ok(_) => UserResponse {
                claim: None,
                token: None,
                status: "success".to_string(),
            },
            Err(e) => UserResponse {
                claim: None,
                token: None,
                status: format!("failed create account, reason: {}", e.to_string()),
            },
        }
    }

    pub async fn login(secret: String, user: User, db: PgPool) -> UserResponse {
        let account = sqlx::query_as!(
            User,
            r#"SELECT username, password, role FROM "user" WHERE username = $1"#,
            user.username,
        )
        .fetch_one(&db)
        .await;

        let account = account.unwrap();
        if Auth::verify(account.password.unwrap(), user.password.unwrap().as_bytes()) {
            let user_claims = Claims {
                sub: account.username,
                role: account.role,
                exp: 10000000000,
            };
            let token = match encode(
                &Header::default(),
                &user_claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            ) {
                Ok(t) => t,
                Err(_) => panic!(), // in practice you would return the error
            };
            return UserResponse {
                claim: Some(user_claims),
                token: Some(token),
                status: "success".to_string(),
            };
        }
        return UserResponse {
            claim: None,
            token: None,
            status: "failed".to_string(),
        };
    }

    pub async fn user_list(db: PgPool) -> Vec<User> {
        let users = sqlx::query(r#"SELECT username, role FROM "user""#)
        .map(|row: PgRow| User {
            username: row.get(0),
            role: row.get(1),
            password: None,
        })
        .fetch_all(&db)
        .await;

        users.unwrap_or(vec![])
        
    }

    pub async fn modify_user_role(user: User, db: PgPool) -> Result<()> {
        sqlx::query!(r#"UPDATE "user" SET role = $1 WHERE username = $2"#, user.role, user.username).execute(&db).await?;
        Ok(())
    }

    pub fn validate(secret: String, token: String) -> Option<Claims> {
        match decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        ) {
            Ok(claim) => Some(claim.claims),
            Err(_) => None,
        }
    }

    fn hash(password: &[u8]) -> String {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();
        return argon2::hash_encoded(password, &salt, &config).unwrap();
    }

    fn verify(hash: String, password: &[u8]) -> bool {
        return argon2::verify_encoded(&hash, password).unwrap();
    }
}
