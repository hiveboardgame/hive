DROP INDEX IF EXISTS games_timeout_at_idx;
ALTER TABLE games DROP COLUMN timeout_at;
