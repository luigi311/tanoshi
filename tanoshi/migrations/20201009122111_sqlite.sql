-- Add migration script here
CREATE TABLE manga (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    author TEXT,
    genre TEXT,
    status TEXT,
    description TEXT,
    path TEXT NOT NULL,
    cover_url TEXT NOT NULL,
    is_favorite BOOLEAN NOT NULL DEFAULT false,
    last_read_chapter INTEGER,
    date_added TIMESTAMP NOT NULL,
    UNIQUE (source_id, path)
);

CREATE TABLE chapter (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL,
    manga_id INTEGER NOT NULL,
    title TEXT,
    path TEXT NOT NULL,
    rank INTEGER NOT NULL,
    read_at TIMESTAMP,
    uploaded TIMESTAMP NOT NULL,
    date_added TIMESTAMP NOT NULL,
    UNIQUE (source_id, path)
);

CREATE TABLE page (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL,
    manga_id INTEGER NOT NULL,
    chapter_id INTEGER NOT NULL,
    rank INTEGER NOT NULL,
    url TEXT NOT NULL,
    read_at TIMESTAMP,
    date_added TIMESTAMP NOT NULL,
    UNIQUE (source_id, url)
);