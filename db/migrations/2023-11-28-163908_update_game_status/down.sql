UPDATE games
SET game_status = CASE
    WHEN game_status = 'Finished(0-1)' THEN 'Finished(Winner(b))'
    WHEN game_status = 'Finished(1-0)' THEN 'Finished(Winner(w))'
    WHEN game_status = 'Finished(½-½)' THEN 'Finished(Draw)'
    ELSE game_status
END;
