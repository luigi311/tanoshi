DROP TABLE IF EXISTS history;
DROP TABLE IF EXISTS favorite;
DROP TABLE IF EXISTS "user";
DROP TABLE IF EXISTS page;
DROP TABLE IF EXISTS chapter;
DROP TABLE IF EXISTS manga;
DROP TABLE IF EXISTS source;

CREATE TABLE "user"
(
    id       SERIAL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    created  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX username_idx ON "user" (username);

CREATE TABLE source
(
    id      SERIAL PRIMARY KEY,
    name    TEXT NOT NULL,
    url     TEXT NOT NULL,
    version VARCHAR(8) DEFAULT "1.0",
    created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (name, url)
);

CREATE INDEX source_name_idx ON source (name);

CREATE TABLE manga
(
    id            SERIAL PRIMARY KEY,
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

CREATE INDEX manga_title_idx ON manga (source_id, title);

CREATE TABLE chapter
(
    id       SERIAL PRIMARY KEY,
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

CREATE INDEX chapter_idx ON chapter (manga_id, number);

CREATE TABLE page
(
    id         SERIAL PRIMARY KEY,
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

CREATE INDEX page_idx ON page (chapter_id);

CREATE TABLE history
(
    id         SERIAL PRIMARY KEY,
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

CREATE INDEX history_idx ON history (user_id, chapter_id);

CREATE TABLE favorite
(
    id       SERIAL PRIMARY KEY,
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

CREATE INDEX favorite_idx ON favorite (user_id, manga_id);
