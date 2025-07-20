use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub is_admin: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub telegram_chat_id: Option<i64>,
    pub pushover_user_key: Option<String>,
    pub gotify_token: Option<String>,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: 0,
            username: String::new(),
            password: String::new(),
            is_admin: false,
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
            telegram_chat_id: None,
            pushover_user_key: None,
            gotify_token: None,
        }
    }
}
