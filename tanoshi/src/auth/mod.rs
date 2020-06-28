use serde::{Deserialize, Serialize};

pub mod auth;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub role: String,
    pub telegram_chat_id: Option<i64>,
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
    pub role: String,
    pub exp: usize,
    telegram_chat_id: Option<i64>,
}
