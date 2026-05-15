create index users_email on users (email);
create index users_username on users (username);
drop index games_updated_at_id;
drop index games_users_user_id_game_id;
