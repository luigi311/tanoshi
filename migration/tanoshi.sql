DROP TABLE IF EXISTS user;
DROP TABLE IF EXISTS source;
DROP TABLE IF EXISTS manga;
DROP TABLE IF EXISTS chapter;
DROP TABLE IF EXISTS page;
DROP TABLE IF EXISTS history;
DROP TABLE IF EXISTS favorite;

CREATE TABLE user
(
    id       INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    created  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE source
(
    id      INTEGER PRIMARY KEY,
    name    TEXT NOT NULL,
    url     TEXT NOT NULL,
    created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (name, url)
);

CREATE TABLE manga
(
    id            INTEGER PRIMARY KEY,
    source_id     INTEGER,
    title         TEXT NOT NULL,
    author        TEXT,
    status        TEXT,
    description   TEXT,
    path          TEXT NOT NULL,
    thumbnail_url TEXT NOT NULL,
    created       TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated       TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (source_id, path),
    FOREIGN KEY (source_id)
        REFERENCES source (id)
        ON DELETE CASCADE
        ON UPDATE NO ACTION
);

CREATE TABLE chapter
(
    id       INTEGER PRIMARY KEY,
    manga_id INTEGER,
    title    TEXT,
    number   TEXT NOT NULL,
    path     TEXT NOT NULL,
    uploaded TIMESTAMP,
    created  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (manga_id, path),
    FOREIGN KEY (manga_id)
        REFERENCES manga (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION
);

CREATE TABLE page
(
    id         INTEGER PRIMARY KEY,
    chapter_id INTEGER,
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

CREATE TABLE history
(
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER,
    chapter_id INTEGER   NOT NULL,
    last_page  INTEGER,
    at         TIMESTAMP NOT NULL,
    created    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id)
        REFERENCES user (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION,
    FOREIGN KEY (chapter_id)
        REFERENCES chapter (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION
);

CREATE TABLE favorite
(
    id       INTEGER PRIMARY KEY,
    user_id  INTEGER NOT NULL,
    manga_id INTEGER NOT NULL,
    created  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, manga_id),
    FOREIGN KEY (user_id)
        REFERENCES user (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION,
    FOREIGN KEY (manga_id)
        REFERENCES manga (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION
);

INSERT INTO source(name, url)
VALUES ('mangasee', 'https://mangaseeonline.us');