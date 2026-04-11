CREATE TABLE chat_messages (
    id BIGSERIAL PRIMARY KEY,
    channel_type TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    sender_id UUID NOT NULL REFERENCES users(id),
    recipient_id UUID REFERENCES users(id),
    username TEXT NOT NULL,
    body TEXT NOT NULL,
    turn INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    game_id UUID,
    CONSTRAINT chat_messages_game_id_fkey
        FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    CONSTRAINT chat_messages_channel_shape_check
        CHECK (
            (
                channel_type = 'direct'
                AND recipient_id IS NOT NULL
                AND game_id IS NULL
            )
            OR (
                channel_type IN ('game_players', 'game_spectators')
                AND recipient_id IS NULL
                AND game_id IS NOT NULL
            )
            OR (
                channel_type NOT IN ('direct', 'game_players', 'game_spectators')
                AND recipient_id IS NULL
                AND game_id IS NULL
            )
        ),
    CONSTRAINT chat_messages_body_length_check
        CHECK (char_length(body) <= 1000)
);

CREATE INDEX idx_chat_messages_channel
    ON chat_messages (channel_type, channel_id, created_at DESC);

CREATE INDEX idx_chat_messages_direct_sender_id
    ON chat_messages (sender_id, created_at DESC)
    WHERE channel_type = 'direct';

CREATE INDEX idx_chat_messages_direct_recipient_id
    ON chat_messages (recipient_id, created_at DESC)
    WHERE channel_type = 'direct';

CREATE INDEX idx_chat_messages_game_id
    ON chat_messages (game_id)
    WHERE game_id IS NOT NULL;

CREATE TABLE chat_read_receipts (
    user_id UUID NOT NULL REFERENCES users(id),
    channel_type TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    last_read_at TIMESTAMPTZ NOT NULL,
    game_id UUID,
    PRIMARY KEY (user_id, channel_type, channel_id),
    CONSTRAINT chat_read_receipts_game_id_fkey
        FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    CONSTRAINT chat_read_receipts_game_id_consistency
        CHECK (
            (
                channel_type IN ('game_players', 'game_spectators')
                AND game_id IS NOT NULL
            )
            OR (
                channel_type NOT IN ('game_players', 'game_spectators')
                AND game_id IS NULL
            )
        )
);

CREATE INDEX idx_chat_read_receipts_game_id
    ON chat_read_receipts (game_id)
    WHERE game_id IS NOT NULL;

CREATE TABLE user_blocks (
    blocker_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CHECK (blocker_id != blocked_id)
);

CREATE INDEX idx_user_blocks_blocker ON user_blocks (blocker_id);

CREATE TABLE user_tournament_chat_mutes (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tournament_id UUID NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, tournament_id)
);

CREATE INDEX idx_user_tournament_chat_mutes_user
    ON user_tournament_chat_mutes (user_id);
