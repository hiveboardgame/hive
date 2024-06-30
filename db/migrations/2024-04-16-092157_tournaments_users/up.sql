create table tournaments_users (
  tournament_id uuid references tournaments(id) on delete cascade,
  user_id uuid references users(id),
  primary key(tournament_id, user_id)
);
