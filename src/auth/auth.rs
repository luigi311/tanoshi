use crate::auth::{Claims, User, UserResponse};
use argon2::{self, Config};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand;
use rand::Rng;
use sled::Db;
use std::convert::TryInto;

#[derive(Clone)]
pub struct Auth {
    db: Db,
}

impl Auth {
    pub fn new() -> Self {
        let db = sled::open("./db/auth").unwrap();
        Auth { db }
    }

    pub fn register(&self, user: User) -> UserResponse {
        let hashed = self.hash(user.password.as_bytes());
        let hashed = hashed.as_bytes();
        match self
            .db
            .insert(
                format!("user:{}:password", user.username),
                Vec::from(hashed),
            )
            .unwrap()
        {
            Some(ok) => UserResponse {
                token: "".to_string(),
                status: "username exists".to_string(),
            },
            None => UserResponse {
                token: "".to_string(),
                status: "success".to_string(),
            },
            _ => UserResponse {
                token: "".to_string(),
                status: "failed".to_string(),
            },
        }
    }

    pub fn login(&self, user: User) -> UserResponse {
        let hashed = self
            .db
            .get(format!("user:{}:password", user.username))
            .unwrap()
            .expect("user not exists");

        if self.verify(&hashed, user.password.as_bytes()) {
            let key = b"secretkey";
            let user_claims = Claims {
                sub: user.username,
                company: "tanoshi".to_string(),
                exp: 10000000000,
            };
            let token = match encode(
                &Header::default(),
                &user_claims,
                &EncodingKey::from_secret(key),
            ) {
                Ok(t) => t,
                Err(_) => panic!(), // in practice you would return the error
            };
            return UserResponse {
                token: token,
                status: "success".to_string(),
            };
        } else {
            return UserResponse {
                token: "".to_string(),
                status: "failed".to_string(),
            };
        }
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
