use crate::auth::{Claims, User, UserResponse};
use argon2::{self, Config};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand;
use rand::Rng;
use sqlx;
use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct Auth {}

impl Auth {
    pub async fn register(user: User, db: PgPool) -> UserResponse {
        let hashed = Auth::hash(user.password.as_bytes());
        match sqlx::query!(
            r#"INSERT INTO "user"(username, password) VALUES ($1, $2)"#,
            user.username,
            hashed
        )
        .execute(&db).await
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
            r#"SELECT username, password FROM "user" WHERE username = $1"#,
            user.username,
        )
        .fetch_one(&db)
        .await;

        if Auth::verify(account.unwrap().password, user.password.as_bytes()) {
            let user_claims = Claims {
                sub: user.username,
                company: "tanoshi".to_string(),
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
