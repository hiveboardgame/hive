create index games_white on games (finished, white_id, tournament_id, nanoid);
create index games_black on games (finished, black_id, tournament_id, nanoid);
drop index games_black_player;
drop index games_white_player;
