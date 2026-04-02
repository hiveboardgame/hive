ALTER TABLE chat_messages
    ADD CONSTRAINT chat_messages_body_length_check
    CHECK (char_length(body) <= 1000);
