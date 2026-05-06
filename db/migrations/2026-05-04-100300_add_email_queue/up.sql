create table email_queue (
  id uuid default gen_random_uuid() primary key not null,
  user_id uuid references users(id) on delete cascade, -- null for system emails
  game_id text references games(nanoid) on delete cascade, -- set for per-move notifications; used for dedup
  digest_date date, -- set for digest rows; used for dedup (see index below)
  kind text not null, -- 'verification' | 'password_reset' | 'turn_digest' | ...
  payload jsonb not null, -- structured data; body rendered at send time
  to_address text not null,
  created_at timestamp with time zone not null default now(),
  scheduled_at timestamp with time zone not null default now(),
  attempts smallint not null default 0,
  last_error text,
  sent_at timestamp with time zone -- null until delivered
);

create index email_queue_scheduled_at on email_queue (scheduled_at)
  where sent_at is null and attempts < 3;

-- One unsent notification per (user, game) — prevents pile-ups in per-move mode.
create unique index email_queue_user_game on email_queue (user_id, game_id)
  where sent_at is null and game_id is not null;

-- One digest per user per calendar day. Does NOT filter on sent_at so a sent+committed
-- digest still blocks a re-queue after a crash/restart on the same UTC date.
create unique index email_queue_user_digest on email_queue (user_id, digest_date)
  where kind = 'turn_digest' and digest_date is not null;
