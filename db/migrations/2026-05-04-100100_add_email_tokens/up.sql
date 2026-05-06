create table email_tokens (
  id uuid default gen_random_uuid() primary key not null,
  user_id uuid not null references users(id) on delete cascade,
  purpose text not null, -- 'verify_email' | 'reset_password' | 'change_email'
  token_hash text not null, -- SHA-256 of the plaintext token sent in the link
  created_at timestamp with time zone not null default now(),
  expires_at timestamp with time zone not null,
  used_at timestamp with time zone
);

create index email_tokens_token_hash on email_tokens (token_hash) where used_at is null;
create index email_tokens_user_purpose on email_tokens (user_id, purpose) where used_at is null;
