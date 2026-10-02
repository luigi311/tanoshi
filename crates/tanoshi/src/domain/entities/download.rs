use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct DownloadQueue {
    pub id: i64,
    pub source_id: i64,
    pub source_name: String,
    pub manga_id: i64,
    pub manga_title: String,
    pub chapter_id: i64,
    pub chapter_title: String,
    pub rank: i64,
    pub url: String,
    pub priority: i64,
    pub date_added: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct DownloadQueueEntry {
    pub source_id: i64,
    pub source_name: String,
    pub manga_id: i64,
    pub manga_title: String,
    pub chapter_id: i64,
    pub chapter_title: String,
    pub downloaded: i64,
    pub total: i64,
    pub priority: i64,
    pub date_added: NaiveDateTime,
}

/// A snapshot or a batch of absolute queue states. Versions belong to the
/// current group of subscribers; every new subscription starts with a snapshot.
#[derive(Debug, Clone)]
pub struct DownloadQueueUpdate {
    pub snapshot: bool,
    pub from_version: i64,
    pub version: i64,
    pub updates: Vec<DownloadQueueEntry>,
    pub removed_ids: Vec<i64>,
    pub resync_required: bool,
}

impl DownloadQueueUpdate {
    pub fn resync_required() -> Self {
        Self {
            snapshot: false,
            from_version: 0,
            version: 0,
            updates: vec![],
            removed_ids: vec![],
            resync_required: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadedChapter {
    pub id: i64,
    pub source_id: i64,
    pub manga_id: i64,
    pub title: String,
    pub path: String,
    pub number: f64,
    pub scanlator: String,
    pub uploaded: NaiveDateTime,
    pub date_added: NaiveDateTime,
    pub downloaded_path: Option<String>,
}
