CREATE TABLE IF NOT EXISTS "user"
(
    id       INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    role VARCHAR(8) DEFAULT 'READER',
    created  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS username_idx ON "user" (username);

CREATE TABLE IF NOT EXISTS manga
(
    id            INTEGER PRIMARY KEY,
    source        TEXT NOT NULL,
    title         TEXT NOT NULL,
    author        TEXT,
    status        TEXT,
    description   TEXT,
    path          TEXT NOT NULL,
    thumbnail_url TEXT NOT NULL,
    created       TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated       TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (source, path)
);

CREATE INDEX IF NOT EXISTS manga_source_title_idx ON manga (source, title);

CREATE TABLE IF NOT EXISTS chapter
(
    id       INTEGER PRIMARY KEY,
    user_id  INTEGER,
    source   TEXT NOT NULL,
    manga_id INTEGER NOT NULL,
    title    TEXT,
    volume   TEXT,
    number   TEXT,
    path     TEXT NOT NULL,
    uploaded TIMESTAMP,
    created  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, manga_id, path),
    FOREIGN KEY (user_id)
        REFERENCES "user" (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION,
    FOREIGN KEY (manga_id)
        REFERENCES manga (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION
);

CREATE INDEX IF NOT EXISTS chapter_idx ON chapter (manga_id, volume, number);

CREATE TABLE IF NOT EXISTS page
(
    id         INTEGER PRIMARY KEY,
    source     TEXT NOT NULL,
    chapter_id INTEGER NOT NULL,
    rank       INTEGER NOT NULL,
    url        TEXT    NOT NULL,
    created    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (chapter_id, rank, url),
    FOREIGN KEY (chapter_id)
        REFERENCES chapter (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION
);

CREATE INDEX IF NOT EXISTS page_idx ON page (chapter_id);

CREATE TABLE IF NOT EXISTS history
(
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER   NOT NULL,
    chapter_id INTEGER   NOT NULL,
    last_page  INTEGER,
    at         TIMESTAMP NOT NULL,
    created    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, chapter_id),
    FOREIGN KEY (user_id)
        REFERENCES "user" (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION,
    FOREIGN KEY (chapter_id)
        REFERENCES chapter (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION
);

CREATE INDEX IF NOT EXISTS history_idx ON history (user_id, chapter_id);

CREATE TABLE IF NOT EXISTS favorite
(
    id       INTEGER PRIMARY KEY,
    user_id  INTEGER NOT NULL,
    manga_id INTEGER NOT NULL,
    created  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, manga_id),
    FOREIGN KEY (user_id)
        REFERENCES "user" (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION,
    FOREIGN KEY (manga_id)
        REFERENCES manga (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION
);

CREATE INDEX IF NOT EXISTS favorite_idx ON favorite (user_id, manga_id);