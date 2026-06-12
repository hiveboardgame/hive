CREATE TABLE game_hashes (
    hash BIGINT NOT NULL,
    game_id UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    turn INT NOT NULL,
    rating DOUBLE PRECISION,
    result TEXT NOT NULL,
    speed TEXT NOT NULL,
    game_type TEXT NOT NULL,
    rated BOOL NOT NULL,
    played_at TIMESTAMPTZ NOT NULL,
    -- The move (piece + position notation) that produced this position; human-readable
    -- label only. Suggested moves are keyed by the next turn's `hash`, not by notation.
    move_piece TEXT NOT NULL,
    move_position TEXT NOT NULL,
    -- Total number of turns in the game (denormalized), for filtering out ultra-short games.
    game_length INT NOT NULL,
    PRIMARY KEY (game_id, turn)
);

CREATE INDEX game_hashes_hash_idx ON game_hashes (hash);
