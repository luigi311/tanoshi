BEGIN;

PRAGMA foreign_keys=OFF;

CREATE TABLE IF NOT EXISTS chapter_new
(
    id       INTEGER PRIMARY KEY,
    user_id  INTEGER,
    manga_id INTEGER,
    title    TEXT,
    volume   TEXT,
    number   TEXT,
    path     TEXT NOT NULL,
    uploaded TIMESTAMP,
    created  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, manga_id, path) ON CONFLICT IGNORE,
    FOREIGN KEY (user_id)
        REFERENCES "user" (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION,
    FOREIGN KEY (manga_id)
        REFERENCES manga (id)
        ON DELETE CASCADE
        On UPDATE NO ACTION
);

INSERT INTO chapter_new SELECT * FROM chapter;

DROP TABLE chapter;

ALTER TABLE chapter_new RENAME TO chapter;

PRAGMA foreign_key_check;

COMMIT;