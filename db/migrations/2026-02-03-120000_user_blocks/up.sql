-- Block list: blocker_id does not receive messages from blocked_id (primarily DMs).
CREATE TABLE user_blocks (
    blocker_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CHECK (blocker_id != blocked_id)
);

CREATE INDEX idx_user_blocks_blocker ON user_blocks(blocker_id);
