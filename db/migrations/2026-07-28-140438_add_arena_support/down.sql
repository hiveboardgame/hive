drop index if exists games_arena_game_id;

alter table games drop column finished_at;
alter table games drop column arena_game_id;
alter table games drop column black_berserked;
alter table games drop column white_berserked;
alter table tournaments_users drop column joined_at;
alter table tournaments drop column arena_duration_seconds;
