create table puzzles (
  id uuid default gen_random_uuid() primary key not null,
  nanoid text not null unique,
  kind text not null,
  game_type text not null,
  -- exactly one start: a game history (whose last move is the setup move) or a HOP position
  history text,
  position text,
  solution text[] not null,
  variations jsonb not null default '[]',
  solver_moves integer not null,
  themes text[] not null default '{}',
  authors text[] not null,
  source text not null,
  -- the nanoid of the game the puzzle was mined from; no foreign key, games may be deleted
  source_game text,
  collection text,
  title text,
  hint text,
  level text,
  -- canonical hash of the start position, so the same position is never imported twice
  hash bigint not null,
  rating double precision not null default 1500,
  deviation double precision not null default 350,
  volatility double precision not null default 0.06,
  plays integer not null default 0,
  solves integer not null default 0,
  stars_sum integer not null default 0,
  stars_count integer not null default 0,
  hidden boolean not null default false,
  created_at timestamp with time zone not null default now(),
  constraint puzzles_one_start check ((history is null) <> (position is null)),
  constraint puzzles_credited check (cardinality(authors) >= 1)
);

create unique index puzzles_hash_kind on puzzles (hash, kind);
create index puzzles_visible_rating on puzzles (rating) where not hidden;
create index puzzles_source_game on puzzles (source_game) where source_game is not null;

create table puzzle_attempts (
  id bigserial primary key not null,
  user_id uuid not null references users(id) on delete cascade,
  puzzle_id uuid not null references puzzles(id) on delete cascade,
  solved boolean not null,
  rated boolean not null,
  hinted boolean not null default false,
  time_ms integer,
  rating_before double precision,
  rating_after double precision,
  created_at timestamp with time zone not null default now()
);

create unique index puzzle_attempts_one_rated on puzzle_attempts (user_id, puzzle_id) where rated;
create index puzzle_attempts_by_user on puzzle_attempts (user_id, created_at desc);
create index puzzle_attempts_by_puzzle on puzzle_attempts (puzzle_id);

create table puzzle_stars (
  user_id uuid not null references users(id) on delete cascade,
  puzzle_id uuid not null references puzzles(id) on delete cascade,
  stars smallint not null,
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  primary key (user_id, puzzle_id),
  constraint puzzle_stars_range check (stars between 0 and 5)
);

create table daily_puzzles (
  day date primary key not null,
  puzzle_id uuid not null unique references puzzles(id) on delete cascade,
  created_at timestamp with time zone not null default now()
);

create table daily_puzzle_queue (
  puzzle_id uuid primary key not null references puzzles(id) on delete cascade,
  queued_by uuid references users(id) on delete set null,
  created_at timestamp with time zone not null default now()
);
