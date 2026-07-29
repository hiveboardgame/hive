alter table tournaments_users add column seed int4;
alter table tournaments_users add column rating float8;

create unique index tournaments_users_seed on tournaments_users (tournament_id, seed) where seed is not null;
