CREATE INDEX idx_chat_messages_spectator_sender_id
    ON chat_messages (sender_id, created_at DESC)
    WHERE channel_type = 'game_spectators';
