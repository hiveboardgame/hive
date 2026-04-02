-- Mute tournament lobby: user does not receive live messages or unread for this tournament.
CREATE TABLE user_tournament_chat_mutes (
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tournament_id UUID NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, tournament_id)
);

CREATE INDEX idx_user_tournament_chat_mutes_user ON user_tournament_chat_mutes(user_id);
