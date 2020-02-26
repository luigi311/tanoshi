use crate::auth::{User, UserResponse};
use argon2::{self, Config};
use rand;
use rand::Rng;
use sled::Db;

#[derive(Clone)]
pub struct Auth {
    db: Box<Db>,
}

impl Auth {
    pub fn new(db: Box<Db>) -> Self {
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
                status: "success".to_string(),
            },
            None => UserResponse {
                status: "failed".to_string(),
            },
            _ => UserResponse {
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
            return UserResponse {
                status: "success".to_string(),
            };
        } else {
            return UserResponse {
                status: "failed".to_string(),
            };
        }
    }

    pub fn hash(&self, password: &[u8]) -> String {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();
        return argon2::hash_encoded(password, &salt, &config).unwrap();
    }

    pub fn verify(&self, hash: &[u8], password: &[u8]) -> bool {
        return argon2::verify_encoded(std::str::from_utf8(hash).unwrap(), password).unwrap();
    }
}
