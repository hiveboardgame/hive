DROP INDEX IF EXISTS idx_chat_read_receipts_game_id;
ALTER TABLE chat_read_receipts
    DROP CONSTRAINT IF EXISTS chat_read_receipts_game_id_consistency;
ALTER TABLE chat_read_receipts
    DROP CONSTRAINT IF EXISTS chat_read_receipts_game_id_fkey;
ALTER TABLE chat_read_receipts
    DROP COLUMN IF EXISTS game_id;

DROP INDEX IF EXISTS idx_chat_messages_game_id;
ALTER TABLE chat_messages
    DROP CONSTRAINT IF EXISTS chat_messages_game_id_consistency;
ALTER TABLE chat_messages
    DROP CONSTRAINT IF EXISTS chat_messages_game_id_fkey;
ALTER TABLE chat_messages
    DROP COLUMN IF EXISTS game_id;
