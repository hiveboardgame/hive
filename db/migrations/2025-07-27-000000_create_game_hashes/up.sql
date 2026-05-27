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
    PRIMARY KEY (game_id, turn)
);

CREATE INDEX game_hashes_hash_idx ON game_hashes (hash);
