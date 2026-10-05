use chrono::NaiveDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryCursor {
    pub read_at: NaiveDateTime,
    pub manga_id: i64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HistoryBounds {
    pub after: Option<HistoryCursor>,
    pub before: Option<HistoryCursor>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HistoryPageInfo {
    pub has_previous_page: bool,
    pub has_next_page: bool,
}

#[derive(Debug, Clone)]
pub struct HistoryChapter {
    pub manga_id: i64,
    pub chapter_id: i64,
    pub manga_title: String,
    pub cover_url: String,
    pub chapter_title: String,
    pub read_at: NaiveDateTime,
    pub last_page_read: i64,
    pub is_complete: bool,
    pub source_id: i64,
}
