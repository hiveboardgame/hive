-- Chat messages: durable storage for all chat (game, tournament, DM, global)
CREATE TABLE chat_messages (
    id              BIGSERIAL PRIMARY KEY,
    channel_type    TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    sender_id       UUID NOT NULL REFERENCES users(id),
    username        TEXT NOT NULL,
    body            TEXT NOT NULL,
    turn            INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_chat_messages_channel ON chat_messages (channel_type, channel_id, created_at DESC);
