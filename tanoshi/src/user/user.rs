use async_graphql::{Enum, SimpleObject};
use serde::{Deserialize, Serialize};

#[derive(Debug, Enum, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, SimpleObject)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[graphql(skip)]
    pub password: String,
    pub role: Role,
}
