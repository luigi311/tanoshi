use serde::{Deserialize, Serialize};

pub mod auth;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Role {
    Admin,
    Reader,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub username: String,
    pub password: String,
    pub role: Role,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claims>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Claims {
    pub sub: String,
    pub role: Role,
    pub exp: usize,
}
