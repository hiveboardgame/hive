DROP TABLE game_hashes;
-- Reset the array the backfill keys off of, so a fresh `up` + backfill repopulates everything.
UPDATE games SET hashes = '{}';
