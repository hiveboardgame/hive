CREATE TABLE chat_channels (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL,
    lookup_key TEXT NOT NULL,
    direct_user_low_id UUID REFERENCES users(id) ON DELETE CASCADE,
    direct_user_high_id UUID REFERENCES users(id) ON DELETE CASCADE,
    game_id UUID REFERENCES games(id) ON DELETE CASCADE,
    tournament_id UUID REFERENCES tournaments(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chat_channels_unique_kind_lookup_key
        UNIQUE (kind, lookup_key),
    CONSTRAINT chat_channels_direct_users_order_check
        CHECK (
            direct_user_low_id IS NULL
            OR direct_user_high_id IS NULL
            OR direct_user_low_id < direct_user_high_id
        ),
    CONSTRAINT chat_channels_shape_check
        CHECK (
            (
                kind = 'direct'
                AND lookup_key = direct_user_low_id::text || ':' || direct_user_high_id::text
                AND direct_user_low_id IS NOT NULL
                AND direct_user_high_id IS NOT NULL
                AND game_id IS NULL
                AND tournament_id IS NULL
            )
            OR (
                kind IN ('game_players', 'game_spectators')
                AND lookup_key = game_id::text
                AND direct_user_low_id IS NULL
                AND direct_user_high_id IS NULL
                AND game_id IS NOT NULL
                AND tournament_id IS NULL
            )
            OR (
                kind = 'tournament_lobby'
                AND lookup_key = tournament_id::text
                AND direct_user_low_id IS NULL
                AND direct_user_high_id IS NULL
                AND game_id IS NULL
                AND tournament_id IS NOT NULL
            )
            OR (
                kind = 'global'
                AND lookup_key = 'global'
                AND direct_user_low_id IS NULL
                AND direct_user_high_id IS NULL
                AND game_id IS NULL
                AND tournament_id IS NULL
            )
        )
);

CREATE UNIQUE INDEX chat_channels_unique_direct_users
    ON chat_channels (direct_user_low_id, direct_user_high_id)
    WHERE kind = 'direct';

CREATE UNIQUE INDEX chat_channels_unique_game
    ON chat_channels (kind, game_id)
    WHERE kind IN ('game_players', 'game_spectators');

CREATE UNIQUE INDEX chat_channels_unique_tournament_lobby
    ON chat_channels (tournament_id)
    WHERE kind = 'tournament_lobby';

CREATE UNIQUE INDEX chat_channels_unique_global
    ON chat_channels (kind)
    WHERE kind = 'global';

INSERT INTO chat_channels (kind, lookup_key)
VALUES ('global', 'global')
ON CONFLICT (kind, lookup_key) DO NOTHING;

CREATE TABLE chat_messages (
    id BIGSERIAL PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES chat_channels(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    turn INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chat_messages_body_length_check
        CHECK (char_length(body) BETWEEN 1 AND 1000)
);

CREATE INDEX idx_chat_messages_channel_history
    ON chat_messages (channel_id, id DESC);

CREATE TABLE chat_read_receipts (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL REFERENCES chat_channels(id) ON DELETE CASCADE,
    last_read_message_id BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, channel_id)
);

CREATE TABLE user_blocks (
    blocker_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CHECK (blocker_id != blocked_id)
);

CREATE TABLE user_tournament_chat_mutes (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tournament_id UUID NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, tournament_id)
);
