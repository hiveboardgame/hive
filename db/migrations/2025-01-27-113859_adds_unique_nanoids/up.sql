drop index tournament_nanoid;
drop index games_nanoid;

create unique index games_nanoid on games (nanoid);
create unique index tournaments_nanoid on tournaments (nanoid);
create unique index challenges_nanoid on challenges (nanoid);
