create table email_queue (
  id uuid default gen_random_uuid() primary key not null,
  user_id uuid references users(id) on delete cascade, -- null for system emails
  kind text not null, -- 'verification' | 'password_reset' | 'email_already_registered' | 'email_changed_notice'
  payload jsonb not null, -- structured data (incl. plaintext token); body rendered at send time
  to_address text not null,
  created_at timestamp with time zone not null default now(),
  scheduled_at timestamp with time zone not null default now(),
  attempts smallint not null default 0,
  last_error text,
  sent_at timestamp with time zone -- null until delivered
);

create index email_queue_scheduled_at on email_queue (scheduled_at)
  where sent_at is null and attempts < 3;
