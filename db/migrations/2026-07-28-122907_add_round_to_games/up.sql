alter table games add column round int4;

create index games_tournament_round on games (tournament_id, round) where tournament_id is not null;
