-- Read receipts: per-user last-read timestamp per channel (for unread counts)
CREATE TABLE chat_read_receipts (
    user_id         UUID NOT NULL REFERENCES users(id),
    channel_type    TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    last_read_at    TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, channel_type, channel_id)
);
