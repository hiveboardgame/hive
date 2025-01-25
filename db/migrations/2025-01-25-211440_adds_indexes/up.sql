create index users_username on users (username);
create index users_email on users (email);
create index games_nanoid on games (nanoid);
create index games_tournament on games (tournament_id);
create index games_white on games (finished, white_id, tournament_id, nanoid);
create index games_black on games (finished, black_id, tournament_id, nanoid);
create index ratings_user on ratings (user_uid);
create index ratings_rating on ratings (rating);
create index tournament_nanoid on tournaments (nanoid);
