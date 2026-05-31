create index games_users_user_id_game_id on games_users (user_id, game_id);
create index games_updated_at_id on games (updated_at desc, id desc);
drop index users_username;
drop index users_email;
