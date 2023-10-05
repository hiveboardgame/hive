create table games_users (
  game_id uuid references games(id) on delete cascade,
  user_id uuid references users(id),
  primary key(game_id, user_id)
);
