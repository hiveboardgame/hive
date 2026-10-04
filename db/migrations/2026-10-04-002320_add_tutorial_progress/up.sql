create table tutorial_progress (
  user_id uuid not null references users(id) on delete cascade,
  lesson_id text not null,
  completed_at timestamp with time zone not null default now(),
  primary key (user_id, lesson_id)
);
