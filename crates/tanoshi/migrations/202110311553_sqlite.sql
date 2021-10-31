CREATE TABLE download_queue (
    source_name INTEGER NOT NULL,
    manga_title TEXT NOT NULL,
    chapter_title TEXT NOT NULL,
    rank INTEGER NOT NULL,
    url TEXT NOT NULL,
    state INTEGER NOT NULL,
    date_added TIMESTAMP NOT NULL,
    UNIQUE (url)
);