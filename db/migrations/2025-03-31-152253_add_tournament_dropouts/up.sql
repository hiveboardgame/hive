CREATE TABLE tournament_dropouts (
    tournament_id UUID NOT NULL REFERENCES tournaments(id),
    user_id UUID NOT NULL REFERENCES users(id),
    dropped_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    dropped_by UUID NOT NULL REFERENCES users(id),
    dropped_in_round INTEGER NOT NULL,
    reason TEXT,
    PRIMARY KEY (tournament_id, user_id),
    FOREIGN KEY (tournament_id, user_id) REFERENCES tournaments_users(tournament_id, user_id)
);

CREATE INDEX idx_tournament_dropouts_tournament ON tournament_dropouts(tournament_id);
