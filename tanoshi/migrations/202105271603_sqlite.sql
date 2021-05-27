ALTER TABLE chapter
ADD COLUMN pages JSON DEFAULT '[]';
UPDATE chapter
SET pages = (
        SELECT JSON_GROUP_ARRAY(url)
        FROM page
        WHERE chapter_id = chapter.id
        GROUP BY chapter_id
        ORDER BY "rank"
    );