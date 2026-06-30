create table email_request_log (
  id uuid default gen_random_uuid() primary key not null,
  email text not null,
  ip text not null,
  purpose text not null,
  created_at timestamp with time zone not null default now()
);

create index email_request_log_email on email_request_log (email, purpose, created_at);
create index email_request_log_ip on email_request_log (ip, purpose, created_at);
