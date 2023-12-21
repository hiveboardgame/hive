create table ratings (
  -- only PLM will be rated (for now?)
  id int generated always as identity primary key,
  user_uid uuid references users(id) on delete cascade not null,
  -- corr, rapid, blitz, ...
  -- game_speed text not null,
  played int8 not null default 0,
  won int8 not null default 0,
  lost int8 not null default 0,
  draw int8 not null default 0,
  rating float8 not null default 1500.0,
  deviation float8 not null default 350.0,
  volatility float8 not null default 0.06,
  created_at timestamp with time zone not null,
  updated_at timestamp with time zone not null
);
