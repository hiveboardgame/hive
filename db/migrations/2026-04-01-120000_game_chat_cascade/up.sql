-- Tie persisted game chat to games.id so game deletion removes both messages and read receipts.

ALTER TABLE chat_messages
    ADD COLUMN game_id UUID;

UPDATE chat_messages AS cm
SET game_id = g.id
FROM games AS g
WHERE cm.channel_type IN ('game_players', 'game_spectators')
  AND cm.channel_id = g.nanoid;

DELETE FROM chat_messages
WHERE channel_type IN ('game_players', 'game_spectators')
  AND game_id IS NULL;

ALTER TABLE chat_messages
    ADD CONSTRAINT chat_messages_game_id_fkey
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE;

ALTER TABLE chat_messages
    ADD CONSTRAINT chat_messages_game_id_consistency
    CHECK (
        (
            channel_type IN ('game_players', 'game_spectators')
            AND game_id IS NOT NULL
        )
        OR (
            channel_type NOT IN ('game_players', 'game_spectators')
            AND game_id IS NULL
        )
    );

CREATE INDEX idx_chat_messages_game_id
    ON chat_messages (game_id)
    WHERE game_id IS NOT NULL;

ALTER TABLE chat_read_receipts
    ADD COLUMN game_id UUID;

UPDATE chat_read_receipts AS crr
SET game_id = g.id
FROM games AS g
WHERE crr.channel_type IN ('game_players', 'game_spectators')
  AND crr.channel_id = g.nanoid;

DELETE FROM chat_read_receipts
WHERE channel_type IN ('game_players', 'game_spectators')
  AND game_id IS NULL;

ALTER TABLE chat_read_receipts
    ADD CONSTRAINT chat_read_receipts_game_id_fkey
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE;

ALTER TABLE chat_read_receipts
    ADD CONSTRAINT chat_read_receipts_game_id_consistency
    CHECK (
        (
            channel_type IN ('game_players', 'game_spectators')
            AND game_id IS NOT NULL
        )
        OR (
            channel_type NOT IN ('game_players', 'game_spectators')
            AND game_id IS NULL
        )
    );

CREATE INDEX idx_chat_read_receipts_game_id
    ON chat_read_receipts (game_id)
    WHERE game_id IS NOT NULL;
