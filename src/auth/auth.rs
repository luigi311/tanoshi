use crate::auth::{Claims, User, UserResponse};
use argon2::{self, Config};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand;
use rand::Rng;
use sled::Db;

#[derive(Clone)]
pub struct Auth {
    secret: String,
}

impl Auth {
    pub fn new() -> Self {
        let secret = std::env::var("TOKEN_SECRET_KEY").unwrap();
        Auth { secret }
    }

    pub fn register(&self, user: User, db: Db) -> UserResponse {
        let hashed = self.hash(user.password.as_bytes());
        let hashed = hashed.as_bytes();
        match db
            .insert(
                format!("user:{}:password", user.username),
                Vec::from(hashed),
            )
            .unwrap()
        {
            Some(_) => UserResponse {
                claim: None,
                token: None,
                status: "success".to_string(),
            },
            None => UserResponse {
                claim: None,
                token: None,
                status: "failed".to_string(),
            },
        }
    }

    pub fn login(&self, user: User, db: Db) -> UserResponse {
        let hashed = db
            .get(format!("user:{}:password", user.username))
            .unwrap()
            .expect("user not exists");

        if self.verify(&hashed, user.password.as_bytes()) {
            let user_claims = Claims {
                sub: user.username,
                company: "tanoshi".to_string(),
                exp: 10000000000,
            };
            let token = match encode(
                &Header::default(),
                &user_claims,
                &EncodingKey::from_secret(self.secret.as_bytes()),
            ) {
                Ok(t) => t,
                Err(_) => panic!(), // in practice you would return the error
            };
            return UserResponse {
                claim: Some(user_claims),
                token: Some(token),
                status: "success".to_string(),
            };
        } else {
            return UserResponse {
                claim: None,
                token: None,
                status: "failed".to_string(),
            };
        }
    }

    pub fn validate(&self, token: String) -> Option<Claims> {
        let claim = {
            if let Ok(claim) = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(self.secret.as_bytes()),
                &Validation::default(),
            ) {
                Some(claim.claims)
            } else {
                None
            }
        };
        claim
    }

    fn hash(&self, password: &[u8]) -> String {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();
        return argon2::hash_encoded(password, &salt, &config).unwrap();
    }

    fn verify(&self, hash: &[u8], password: &[u8]) -> bool {
        return argon2::verify_encoded(std::str::from_utf8(hash).unwrap(), password).unwrap();
    }
}
