create table games_users (
  game_id int references games(id) on delete cascade,
  user_uid text references users(uid),
  primary key(game_id, user_uid)
);
