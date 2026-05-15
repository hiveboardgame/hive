create index games_white_player on games (white_id, finished, speed);
create index games_black_player on games (black_id, finished, speed);
drop index games_white;
drop index games_black;
