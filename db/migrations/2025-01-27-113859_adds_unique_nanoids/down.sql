drop index games_nanoid;
drop index tournaments_nanoid;
drop index challenges_nanoid;

create index tournament_nanoid on tournaments (nanoid);
create index games_nanoid on games (nanoid);
