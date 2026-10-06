-- Keep history pagination independent of the number of chapters a user has read.
-- The denormalized manga ID also lets deletion fallback seek within one manga.
ALTER TABLE user_history ADD COLUMN manga_id INTEGER
    REFERENCES manga(id) ON DELETE CASCADE ON UPDATE NO ACTION;

-- Normalize legacy local timestamps to UTC without discarding nanoseconds.
-- Pass the date/time and timezone suffix to datetime(), with the fraction
-- removed so SQLite cannot round it into the next second. Only pad the leading
-- fractional digits; an offset or Z must never become part of the fraction.
UPDATE user_history
SET manga_id = (
        SELECT c.manga_id FROM chapter c JOIN manga m ON m.id = c.manga_id
        WHERE c.id = user_history.chapter_id
    ),
    read_at = datetime(substr(read_at, 1, 19) ||
            ltrim(substr(read_at, 20), '.0123456789')) || '.' ||
        substr(CASE WHEN substr(read_at, 20, 1) = '.' THEN
            substr(read_at, 21, length(substr(read_at, 21)) -
                length(ltrim(substr(read_at, 21), '0123456789')))
            ELSE '' END || '000000000', 1, 9);

CREATE INDEX idx_user_history_manga_read_at_chapter
ON user_history(user_id, manga_id, read_at DESC, chapter_id DESC);

CREATE TABLE user_manga_history (
    user_id INTEGER NOT NULL REFERENCES user(id) ON DELETE CASCADE,
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    chapter_id INTEGER NOT NULL,
    read_at TIMESTAMP NOT NULL,
    PRIMARY KEY(user_id, manga_id)
);

CREATE INDEX idx_user_manga_history_read_at_manga
ON user_manga_history(user_id, read_at DESC, manga_id DESC);

INSERT INTO user_manga_history(user_id, manga_id, chapter_id, read_at)
SELECT user_id, manga_id, chapter_id, read_at
FROM (
    SELECT h.user_id, h.manga_id, h.chapter_id, h.read_at,
        ROW_NUMBER() OVER (
            PARTITION BY h.user_id, h.manga_id
            ORDER BY h.read_at DESC, h.chapter_id DESC
        ) AS position
    FROM user_history h JOIN user u ON u.id = h.user_id
    WHERE h.manga_id IS NOT NULL
)
WHERE position = 1;

-- These triggers cover normal reading, bulk mark-read/unread, manga migration,
-- and foreign-key cascades in the transaction that changes the source history.
CREATE TRIGGER user_history_latest_insert AFTER INSERT ON user_history
BEGIN
    UPDATE user_history
    SET manga_id = (
            SELECT c.manga_id FROM chapter c JOIN manga m ON m.id = c.manga_id
            WHERE c.id = NEW.chapter_id
        ),
        read_at = datetime(substr(NEW.read_at, 1, 19) ||
                ltrim(substr(NEW.read_at, 20), '.0123456789')) || '.' ||
            substr(CASE WHEN substr(NEW.read_at, 20, 1) = '.' THEN
                substr(NEW.read_at, 21, length(substr(NEW.read_at, 21)) -
                    length(ltrim(substr(NEW.read_at, 21), '0123456789')))
                ELSE '' END || '000000000', 1, 9)
    WHERE rowid = NEW.rowid;
END;

CREATE TRIGGER user_history_latest_update
AFTER UPDATE OF user_id, chapter_id, manga_id, read_at ON user_history
BEGIN
    -- A caller may update chapter_id without knowing the denormalized manga ID.
    -- The guard also terminates this trigger's normalization under recursion.
    UPDATE user_history
    SET manga_id = (
            SELECT c.manga_id FROM chapter c JOIN manga m ON m.id = c.manga_id
            WHERE c.id = NEW.chapter_id
        ),
        read_at = datetime(substr(NEW.read_at, 1, 19) ||
                ltrim(substr(NEW.read_at, 20), '.0123456789')) || '.' ||
            substr(CASE WHEN substr(NEW.read_at, 20, 1) = '.' THEN
                substr(NEW.read_at, 21, length(substr(NEW.read_at, 21)) -
                    length(ltrim(substr(NEW.read_at, 21), '0123456789')))
                ELSE '' END || '000000000', 1, 9)
    WHERE rowid = NEW.rowid AND (
        manga_id IS NOT (
            SELECT c.manga_id FROM chapter c JOIN manga m ON m.id = c.manga_id
            WHERE c.id = NEW.chapter_id
        ) OR read_at IS NOT (datetime(substr(NEW.read_at, 1, 19) ||
                ltrim(substr(NEW.read_at, 20), '.0123456789')) || '.' ||
            substr(CASE WHEN substr(NEW.read_at, 20, 1) = '.' THEN
                substr(NEW.read_at, 21, length(substr(NEW.read_at, 21)) -
                    length(ltrim(substr(NEW.read_at, 21), '0123456789')))
                ELSE '' END || '000000000', 1, 9))
    );

    DELETE FROM user_manga_history
    WHERE user_id = OLD.user_id AND manga_id = OLD.manga_id
        AND chapter_id = OLD.chapter_id
        AND NOT EXISTS(SELECT 1 FROM user_history
            WHERE user_id = OLD.user_id AND manga_id = OLD.manga_id);

    INSERT INTO user_manga_history(user_id, manga_id, chapter_id, read_at)
    SELECT h.user_id, h.manga_id, h.chapter_id, h.read_at
    FROM user_history h JOIN user u ON u.id = h.user_id JOIN manga m ON m.id = h.manga_id
    WHERE h.user_id = OLD.user_id AND h.manga_id = OLD.manga_id
    ORDER BY h.read_at DESC, h.chapter_id DESC LIMIT 1
    ON CONFLICT(user_id, manga_id) DO UPDATE SET
        chapter_id = excluded.chapter_id, read_at = excluded.read_at
    WHERE (user_manga_history.chapter_id, user_manga_history.read_at)
        IS NOT (excluded.chapter_id, excluded.read_at);

    INSERT INTO user_manga_history(user_id, manga_id, chapter_id, read_at)
    SELECT h.user_id, h.manga_id, h.chapter_id, h.read_at
    FROM user_history h JOIN user u ON u.id = h.user_id JOIN manga m ON m.id = h.manga_id
    WHERE h.user_id = NEW.user_id AND h.manga_id = (
        SELECT manga_id FROM user_history WHERE rowid = NEW.rowid
    )
        AND (OLD.user_id IS NOT NEW.user_id OR OLD.manga_id IS NOT h.manga_id)
    ORDER BY h.read_at DESC, h.chapter_id DESC LIMIT 1
    ON CONFLICT(user_id, manga_id) DO UPDATE SET
        chapter_id = excluded.chapter_id, read_at = excluded.read_at
    WHERE (user_manga_history.chapter_id, user_manga_history.read_at)
        IS NOT (excluded.chapter_id, excluded.read_at);
END;

CREATE TRIGGER user_history_latest_delete AFTER DELETE ON user_history
BEGIN
    DELETE FROM user_manga_history
    WHERE user_id = OLD.user_id AND manga_id = OLD.manga_id
        AND chapter_id = OLD.chapter_id
        AND NOT EXISTS(SELECT 1 FROM user_history
            WHERE user_id = OLD.user_id AND manga_id = OLD.manga_id);

    INSERT INTO user_manga_history(user_id, manga_id, chapter_id, read_at)
    SELECT h.user_id, h.manga_id, h.chapter_id, h.read_at
    FROM user_history h JOIN user u ON u.id = h.user_id JOIN manga m ON m.id = h.manga_id
    WHERE h.user_id = OLD.user_id AND h.manga_id = OLD.manga_id
    ORDER BY h.read_at DESC, h.chapter_id DESC LIMIT 1
    ON CONFLICT(user_id, manga_id) DO UPDATE SET
        chapter_id = excluded.chapter_id, read_at = excluded.read_at
    WHERE (user_manga_history.chapter_id, user_manga_history.read_at)
        IS NOT (excluded.chapter_id, excluded.read_at);
END;

-- Chapter refreshes can move a source/path to another manga without rewriting
-- its history. Move the denormalized keys and both summaries in that transaction.
CREATE TRIGGER chapter_latest_history_move AFTER UPDATE OF manga_id ON chapter
WHEN NEW.manga_id IS NOT OLD.manga_id
BEGIN
    UPDATE user_history SET manga_id = (
        SELECT id FROM manga WHERE id = NEW.manga_id
    ) WHERE chapter_id = NEW.id;
END;
