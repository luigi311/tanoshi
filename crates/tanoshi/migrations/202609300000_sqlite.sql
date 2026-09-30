CREATE INDEX idx_download_queue_chapter_id ON download_queue(chapter_id);

-- Match the worker's order, treating both NULL and false downloaded flags as pending.
CREATE INDEX idx_download_queue_order ON download_queue(
    priority, date_added, chapter_id, downloaded IS true, rank
);
