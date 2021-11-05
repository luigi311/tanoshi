#[derive(Debug, Clone)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub icon: String,
    pub need_login: bool,
    pub has_update: bool,
    pub installed: bool,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
    pub telegram_chat_id: Option<i64>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ServerStatus {
    pub activated: bool,
    pub version: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DownloadQueue {
    pub source_name: String,
    pub manga_title: String,
    pub chapter_title: String,
    pub downloaded: i64,
    pub total: i64,
}
