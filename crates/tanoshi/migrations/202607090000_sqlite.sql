CREATE INDEX idx_chapter_manga_id_number ON chapter(manga_id, number);
-- COLLATE NOCASE matches the case-insensitive title comparison used when
-- migrating read progress between sources.
CREATE INDEX idx_chapter_manga_id_title ON chapter(manga_id, title COLLATE NOCASE);
