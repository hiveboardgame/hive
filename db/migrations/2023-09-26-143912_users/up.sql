create table users (
  id uuid default gen_random_uuid() primary key not null,
  username text not null unique,
  password text not null,
  email text not null unique,
  created_at timestamp with time zone not null,
  updated_at timestamp with time zone not null
)
