ALTER TABLE games ADD COLUMN timeout_at TIMESTAMPTZ;

CREATE INDEX games_timeout_at_idx
    ON games (timeout_at)
    WHERE finished = false AND timeout_at IS NOT NULL;

UPDATE games
SET timeout_at = last_interaction + (
    (CASE WHEN turn % 2 = 0 THEN white_time_left ELSE black_time_left END) / 1e9
) * INTERVAL '1 second'
WHERE finished = false
  AND time_mode <> 'Untimed'
  AND game_status = 'InProgress'
  AND last_interaction IS NOT NULL
  AND white_time_left IS NOT NULL
  AND black_time_left IS NOT NULL;
