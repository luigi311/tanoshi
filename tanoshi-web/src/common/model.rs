#[derive(Debug, Clone)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub icon: String,
    pub need_login: bool,
    pub has_update: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Reader,
    Admin,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub role: Role,
}