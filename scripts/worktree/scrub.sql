UPDATE users
SET password = '$argon2id$v=19$m=32768,t=2,p=1$aGl2ZWdhbWUtZGV2LXNhbHQ$bUDG9j9YtwiYQKRxxjyelpjdsiHZCn/8JBlVGi95hsE',
    email = username || '@hivegame.com',
    pending_email = NULL;

TRUNCATE chat_messages, chat_read_receipts;
DELETE FROM chat_channels WHERE kind = 'direct';

TRUNCATE push_devices;
TRUNCATE user_blocks, user_tournament_chat_mutes;
TRUNCATE email_queue, email_tokens, email_request_log;
