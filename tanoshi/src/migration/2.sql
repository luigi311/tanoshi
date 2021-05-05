--- Convert authos from csv to json array
UPDATE manga SET author = (
	SELECT authors FROM (
		WITH split(id, word, str) AS (
    		SELECT id, '', author || ',' FROM manga
    		UNION ALL SELECT id,
    		substr(str, 0, instr(str, ',')),
	    	substr(str, instr(str, ',')+1)
    		FROM split WHERE str!=''
		) SELECT id, json_group_array(word) as authors FROM split WHERE word!='' GROUP BY id) as author_cte
	WHERE (manga.id = author_cte.id) AND (author_cte.id = manga.id)
);


--- Add genre column
ALTER TABLE manga 
ADD COLUMN genre text;
