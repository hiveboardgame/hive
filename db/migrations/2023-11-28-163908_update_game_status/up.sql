UPDATE games
SET game_status = CASE
    WHEN game_status = 'Finished(Winner(b))' THEN 'Finished(0-1)'
    WHEN game_status = 'Finished(Winner(w))' THEN 'Finished(1-0)'
    WHEN game_status = 'Finished(Draw)' THEN 'Finished(½-½)'
    ELSE game_status
END;