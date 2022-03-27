CREATE TABLE tracker_manga (
    id INTEGER PRIMARY KEY,
    tracker VARCHAR(256) NOT NULL,
    tracker_manga_id VARCHAR(256),
    manga_id INTEGER NOT NULL,
    UNIQUE(manga_id, tracker),
    UNIQUE(tracker, tracker_manga_id),
    FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE
);