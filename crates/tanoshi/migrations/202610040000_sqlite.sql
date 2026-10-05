CREATE INDEX idx_chapter_downloaded_date_added_id
ON chapter(date_added, id)
WHERE downloaded_path IS NOT NULL;
