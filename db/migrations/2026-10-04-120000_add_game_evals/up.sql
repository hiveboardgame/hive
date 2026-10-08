create table game_evals (
  id uuid default gen_random_uuid() primary key not null,
  game_id uuid not null references games(id) on delete cascade,
  -- set null keeps a finished eval on the game after its requester's account is gone
  requested_by uuid references users(id) on delete set null,
  status text not null default 'queued',
  moves integer not null,
  progress_pct smallint not null default 0,
  attempts smallint not null default 0,
  worker text,
  engine text,
  result jsonb,
  error text,
  created_at timestamp with time zone not null default now(),
  started_at timestamp with time zone,
  heartbeat_at timestamp with time zone,
  finished_at timestamp with time zone,
  constraint game_evals_status_check
    check (status in ('queued', 'running', 'done', 'failed')),
  constraint game_evals_progress_check
    check (progress_pct between 0 and 100),
  constraint game_evals_result_iff_done_check
    check ((status = 'done') = (result is not null))
);

create unique index game_evals_game_id on game_evals (game_id);

create unique index game_evals_one_active_per_user on game_evals (requested_by)
  where status in ('queued', 'running');

create index game_evals_queue on game_evals (created_at)
  where status = 'queued';

create index game_evals_recent_done on game_evals (finished_at)
  where status = 'done';
