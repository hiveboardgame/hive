create table ratings (
  id int generated always as identity primary key,
  user_uid uuid references users(id) on delete cascade not null,
  -- only PLM will be rated (for now?)
  played int8 not null default 0,
  won int8 not null default 0,
  lost int8 not null default 0,
  draw int8 not null default 0,
  rating float8 not null default 1500.0,
  deviation float8 not null default 350.0,
  volatility float8 not null default 0.06
);
