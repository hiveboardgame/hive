-- Production rollback restores the pre-migration backup. This down migration
-- returns disposable development databases to the legacy schema but cannot
-- reconstruct tournament facts removed by the up migration.
drop table tournament_final_arena_results;
drop table arena_game_results;

update tournaments
set featured_game_id = null
where featured_game_id is not null;

alter table tournaments
    drop constraint tournaments_featured_game_fkey;

alter table games
    drop constraint games_tournament_identity_unique;

update games
set tournament_id = null,
    tournament_slot_id = null,
    arena_ordinal = null
where tournament_id is not null;
delete from tournaments;

-- Tournament deletion cascades through deferred ownership constraints. Force
-- those trigger events to settle before altering their referenced tables.
set constraints all immediate;

drop table schedule_offers;
drop table tournament_final_outcomes;
drop table tournament_elimination_nodes;
drop table tournament_swiss_rounds;
drop table tournaments_organizer_invitations;
drop function enforce_schedule_offer_slot_proposer();

drop index games_arena_move_due_at;
drop index games_tournament;
drop index games_white;
drop index games_black;
drop index games_arena_ordinal_unique;
drop index games_one_per_tournament_slot;

alter table games
    drop constraint games_black_tournament_membership,
    drop constraint games_white_tournament_membership,
    drop constraint games_tournament_slot_fkey,
    drop constraint games_tournament_id_fkey,
    drop constraint games_tournament_ownership_shape,
    drop constraint games_arena_ordinal_check,
    drop constraint games_berserk_requires_arena,
    drop constraint games_arena_deadline_shape,
    drop constraint games_finished_at_requires_terminal,
    drop column arena_ordinal,
    drop column tournament_slot_id,
    drop column arena_move_due_at,
    drop column finished_at,
    drop column black_berserked,
    drop column white_berserked;

drop table tournament_slots;

alter table tournaments_invitations
    drop constraint tournaments_invitations_declined_time_check,
    drop column declined_at;

drop index tournaments_users_accepted_order;
drop index tournaments_users_pairing_number_unique;

alter table tournaments_users
    drop constraint tournaments_users_withdrawal_time_check,
    drop constraint tournaments_users_arena_waiting_check,
    drop constraint tournaments_users_arena_pairing_intent_check,
    drop constraint tournaments_users_pairing_number_check,
    drop column withdrawn_at,
    drop column arena_waiting_since,
    drop column arena_pairing_intent,
    drop column arena_rating,
    drop column pairing_number,
    drop column accepted_at;

drop index tournaments_starts_at_due;

alter table tournaments
    alter column description set not null,
    alter column seats set not null,
    drop constraint tournaments_rating_band_check,
    drop constraint tournaments_featured_game_format_check,
    drop constraint tournaments_seats_check,
    drop constraint tournaments_configuration_check,
    drop constraint tournaments_timestamp_status_check,
    drop constraint tournaments_start_setup_check,
    drop constraint tournaments_bracket_order_check,
    drop column featured_game_id,
    drop column finished_at,
    drop column configuration,
    drop column start_setup,
    drop column bracket_order,
    add column status text not null,
    add column scoring text not null,
    add column tiebreaker text[] not null default '{}',
    add column rounds int4 not null,
    add column mode text not null,
    add column time_mode text not null,
    add column time_base int4,
    add column time_increment int4,
    add column start_mode text not null,
    add column ends_at timestamptz,
    add column round_duration int4;

create index games_tournament on games (tournament_id);
create index games_white on games (finished, white_id, tournament_id, nanoid);
create index games_black on games (finished, black_id, tournament_id, nanoid);

create table schedules (
    id uuid primary key not null default gen_random_uuid(),
    game_id uuid not null references games(id) on delete cascade,
    tournament_id uuid not null references tournaments(id) on delete cascade,
    proposer_id uuid not null references users(id) on delete cascade,
    opponent_id uuid not null references users(id) on delete cascade,
    start_t timestamptz not null,
    agreed boolean not null default false,
    notified boolean not null default false
);
