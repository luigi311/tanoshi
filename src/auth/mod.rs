use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::error::Error;

pub mod auth;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserResponse {
    pub token: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    company: String,
    exp: usize,
}

pub fn validate(token: String) -> Option<String> {
    let token = decode::<Claims>(
        &token,
        &DecodingKey::from_secret("secretkey".as_ref()),
        &Validation::default(),
    )
    .expect("token error");
    Some(token.claims.sub)
}
