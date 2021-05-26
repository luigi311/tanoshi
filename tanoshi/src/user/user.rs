use async_graphql::{Enum, Object};
use serde::{Deserialize, Serialize};
use sqlx::Encode;

#[derive(Debug, Enum, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, sqlx::Type)]
pub enum Role {
    Reader,
    Admin,
}

impl Default for Role {
    fn default() -> Self {
        Role::Reader
    }
}

impl Into<u8> for Role {
    fn into(self) -> u8 {
        match self {
            Role::Reader => 0,
            Role::Admin => 1
        }
    }
}

impl From<u8> for Role {
    fn from(role: u8) -> Self {
        if role == 1 {
            Role::Admin
        } else {
            Role::Reader
        }
    }
}

pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub role: Role,
}

#[Object]
impl User {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn username(&self) -> String {
        self.username.clone()
    }

    async fn role(&self) -> Role {
        self.role.clone()
    }
}
