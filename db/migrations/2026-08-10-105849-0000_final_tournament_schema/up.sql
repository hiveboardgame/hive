-- The frozen export retains the legacy game-to-tournament association for the
-- importer. Preserve the games themselves while releasing that association so
-- the legacy tournaments can be removed before the new restrictive foreign key
-- is installed.
update games
set tournament_id = null
where tournament_id is not null;

-- Legacy tournament facts are restored by the frozen importer. Keeping the
-- target empty makes that import atomic and prevents a partial SQL conversion
-- from becoming a second source of tournament semantics.
delete from tournaments;

drop table schedules;

drop index games_tournament;
drop index games_white;
drop index games_black;

alter table tournaments
    alter column description drop not null,
    alter column seats drop not null,
    drop column status,
    drop column scoring,
    drop column tiebreaker,
    drop column rounds,
    drop column mode,
    drop column time_mode,
    drop column time_base,
    drop column time_increment,
    drop column start_mode,
    drop column ends_at,
    drop column round_duration,
    add column configuration jsonb not null,
    add column finished_at timestamptz,
    add column featured_game_id uuid,
    add column start_setup jsonb,
    add column bracket_order jsonb,
    add constraint tournaments_start_setup_check
        check (
            start_setup is null or (
                configuration -> 'format' ->> 'format' = 'elimination'
                and started_at is null
                and starts_at is null
                and jsonb_typeof(start_setup) = 'object'
                and start_setup - array['id', 'owner_id', 'opened_at', 'seeded_players']::text[] = '{}'::jsonb
                and jsonb_typeof(start_setup -> 'id') = 'string'
                and jsonb_typeof(start_setup -> 'owner_id') = 'string'
                and jsonb_typeof(start_setup -> 'opened_at') = 'string'
                and jsonb_typeof(start_setup -> 'seeded_players') = 'array'
            ) is true
        ),
    add constraint tournaments_bracket_order_check
        check (
            bracket_order is null or (
                configuration -> 'format' ->> 'format' = 'elimination'
                and jsonb_typeof(bracket_order) = 'array'
            ) is true
        ),
    add constraint tournaments_timestamp_status_check
        check (
            finished_at is null
            or (started_at is not null and finished_at >= started_at)
        ),
    add constraint tournaments_configuration_check
        check (
            (
                jsonb_typeof(configuration) = 'object'
                and configuration - array['bot_admission', 'format']::text[] = '{}'::jsonb
                and configuration ->> 'bot_admission' in ('humans_only', 'humans_and_bots', 'bots_only')
                and jsonb_typeof(configuration -> 'format') = 'object'
                and (configuration -> 'format') - array['format', 'configuration']::text[] = '{}'::jsonb
                and configuration -> 'format' ->> 'format' in ('round_robin', 'swiss', 'elimination', 'arena')
                and jsonb_typeof(configuration -> 'format' -> 'configuration') = 'object'
            ) is true
        ),
    add constraint tournaments_seats_check
        check (
            (
                (
                    configuration -> 'format' ->> 'format' = 'arena'
                    and seats is null
                    and min_seats = 0
                )
                or
                (
                    configuration -> 'format' ->> 'format' <> 'arena'
                    and seats is not null
                    and seats > 0
                    and min_seats >= 0
                    and min_seats <= seats
                )
            ) is true
        ),
    add constraint tournaments_featured_game_format_check
        check (
            (
                featured_game_id is null
                or configuration -> 'format' ->> 'format' = 'arena'
            ) is true
        ),
    add constraint tournaments_rating_band_check
        check (
            band_lower is null or band_upper is null or band_lower <= band_upper
        );

create index tournaments_starts_at_due
    on tournaments (starts_at, id)
    where started_at is null and finished_at is null and starts_at is not null;

create table tournaments_organizer_invitations (
    tournament_id uuid not null references tournaments(id) on delete cascade,
    invitee_id uuid not null references users(id) on delete cascade,
    created_at timestamptz not null default now(),
    primary key (tournament_id, invitee_id)
);

alter table tournaments_users
    add column accepted_at timestamptz not null,
    add column pairing_number int4,
    add column arena_rating int4,
    add column arena_pairing_intent text,
    add column arena_waiting_since timestamptz,
    add column withdrawn_at timestamptz,
    add constraint tournaments_users_pairing_number_check
        check (pairing_number is null or pairing_number >= 0),
    add constraint tournaments_users_arena_pairing_intent_check
        check (arena_pairing_intent is null or arena_pairing_intent in ('enabled', 'paused')),
    add constraint tournaments_users_arena_waiting_check
        check (
            arena_waiting_since is null
            or (
                arena_rating is not null
                and arena_pairing_intent = 'enabled'
                and arena_waiting_since >= accepted_at
            )
        ),
    add constraint tournaments_users_withdrawal_time_check
        check (withdrawn_at is null or withdrawn_at >= accepted_at);

create unique index tournaments_users_pairing_number_unique
    on tournaments_users (tournament_id, pairing_number)
    where pairing_number is not null;
create index tournaments_users_accepted_order
    on tournaments_users (tournament_id, accepted_at, user_id);

alter table tournaments_invitations
    add column declined_at timestamptz,
    add constraint tournaments_invitations_declined_time_check
        check (declined_at is null or declined_at >= created_at);

create table tournament_slots (
    id uuid primary key default gen_random_uuid(),
    tournament_id uuid not null references tournaments(id) on delete cascade,
    native_key jsonb not null,
    white_id uuid not null,
    black_id uuid not null,
    clock jsonb not null,
    resolution jsonb,
    resolved_at timestamptz,
    scheduled_at timestamptz,
    deadline_at timestamptz,
    constraint tournament_slots_native_key_unique unique (tournament_id, native_key),
    constraint tournament_slots_tournament_identity_unique unique (tournament_id, id),
    constraint tournament_slots_game_identity_unique
        unique (tournament_id, id, white_id, black_id),
    constraint tournament_slots_distinct_players check (white_id <> black_id),
    constraint tournament_slots_native_key_check
        check (
            (
                jsonb_typeof(native_key) = 'object'
                and native_key ->> 'format' in ('round_robin', 'swiss', 'elimination')
                and (
                    (
                        native_key ->> 'format' = 'round_robin'
                        and jsonb_typeof(native_key -> 'slot') = 'number'
                        and native_key - array['format', 'slot']::text[] = '{}'::jsonb
                        and native_key ->> 'slot' ~ '^(0|[1-9][0-9]*)$'
                        and (native_key ->> 'slot')::numeric <= 18446744073709551615
                    )
                    or (
                        native_key ->> 'format' = 'swiss'
                        and jsonb_typeof(native_key -> 'slot') = 'object'
                        and native_key - array['format', 'slot']::text[] = '{}'::jsonb
                        and (native_key -> 'slot') - array['round_index', 'pairing_index', 'leg']::text[] = '{}'::jsonb
                        and jsonb_typeof(native_key -> 'slot' -> 'round_index') = 'number'
                        and native_key -> 'slot' ->> 'round_index' ~ '^(0|[1-9][0-9]*)$'
                        and (native_key -> 'slot' ->> 'round_index')::numeric <= 4294967295
                        and jsonb_typeof(native_key -> 'slot' -> 'pairing_index') = 'number'
                        and native_key -> 'slot' ->> 'pairing_index' ~ '^(0|[1-9][0-9]*)$'
                        and (native_key -> 'slot' ->> 'pairing_index')::numeric <= 4294967295
                        and native_key -> 'slot' ->> 'leg' in ('single', 'first', 'second')
                    )
                    or (
                        native_key ->> 'format' = 'elimination'
                        and native_key - array['format', 'node', 'slot']::text[] = '{}'::jsonb
                        and jsonb_typeof(native_key -> 'node') = 'number'
                        and native_key ->> 'node' ~ '^(0|[1-9][0-9]*)$'
                        and (native_key ->> 'node')::numeric <= 18446744073709551615
                        and jsonb_typeof(native_key -> 'slot') = 'number'
                        and native_key ->> 'slot' ~ '^(0|[1-9][0-9]*)$'
                        and (native_key ->> 'slot')::numeric <= 4294967295
                    )
                )
            ) is true
        ),
    constraint tournament_slots_clock_check
        check (
            (
                jsonb_typeof(clock) = 'object'
                and clock - array['mode', 'control']::text[] = '{}'::jsonb
                and clock ->> 'mode' in ('realtime', 'correspondence')
                and jsonb_typeof(clock -> 'control') = 'object'
            ) is true
        ),
    constraint tournament_slots_resolution_check
        check (
            resolution is null
            or (
                (
                    jsonb_typeof(resolution) = 'object'
                    and resolution ->> 'kind' in ('result', 'withdrawal', 'clinched')
                    and (
                        (
                            resolution ->> 'kind' in ('result', 'withdrawal')
                            and resolution - array['kind', 'outcome']::text[] = '{}'::jsonb
                            and jsonb_typeof(resolution -> 'outcome') = 'string'
                            and resolution ->> 'outcome' in (
                                'played_white_win',
                                'played_draw',
                                'played_black_win',
                                'adjudicated_white_forfeit_win',
                                'adjudicated_black_forfeit_win',
                                'adjudicated_draw',
                                'adjudicated_white_draw_black_forfeit_loss',
                                'adjudicated_white_forfeit_loss_black_draw',
                                'adjudicated_double_forfeit'
                            )
                        )
                        or (
                            resolution ->> 'kind' = 'clinched'
                            and resolution - array['kind', 'outcome']::text[] = '{}'::jsonb
                            and (
                                not resolution ? 'outcome'
                                or jsonb_typeof(resolution -> 'outcome') = 'null'
                            )
                        )
                    )
                ) is true
            )
        ),
    constraint tournament_slots_resolution_time_check
        check ((resolution is null) = (resolved_at is null)),
    constraint tournament_slots_white_membership foreign key (tournament_id, white_id)
        references tournaments_users(tournament_id, user_id)
        deferrable initially deferred,
    constraint tournament_slots_black_membership foreign key (tournament_id, black_id)
        references tournaments_users(tournament_id, user_id)
        deferrable initially deferred
);

alter table games
    add column white_berserked boolean not null default false,
    add column black_berserked boolean not null default false,
    add column finished_at timestamptz,
    add column arena_move_due_at timestamptz,
    add column tournament_slot_id uuid,
    add column arena_ordinal int8,
    add constraint games_finished_at_requires_terminal
        check (finished_at is null or finished),
    add constraint games_arena_deadline_shape
        check (
            arena_move_due_at is null
            or (not finished and game_start = 'Arena' and timeout_at is null)
        ),
    add constraint games_berserk_requires_arena
        check (
            (not white_berserked and not black_berserked)
            or game_start = 'Arena'
        ),
    add constraint games_tournament_identity_unique unique (tournament_id, id),
    add constraint games_arena_ordinal_check check (arena_ordinal is null or arena_ordinal >= 0),
    add constraint games_tournament_ownership_shape check (
        (tournament_id is null and tournament_slot_id is null and arena_ordinal is null)
        or (tournament_id is not null and tournament_slot_id is not null and arena_ordinal is null)
        or (tournament_id is not null and tournament_slot_id is null and arena_ordinal is not null)
    ),
    add constraint games_tournament_id_fkey
        foreign key (tournament_id) references tournaments(id) on delete restrict,
    add constraint games_tournament_slot_fkey
        foreign key (tournament_id, tournament_slot_id, white_id, black_id)
        references tournament_slots(tournament_id, id, white_id, black_id)
        on delete restrict
        deferrable initially deferred,
    add constraint games_white_tournament_membership foreign key (tournament_id, white_id)
        references tournaments_users(tournament_id, user_id)
        deferrable initially deferred,
    add constraint games_black_tournament_membership foreign key (tournament_id, black_id)
        references tournaments_users(tournament_id, user_id)
        deferrable initially deferred;

alter table tournaments
    add constraint tournaments_featured_game_fkey
        foreign key (id, featured_game_id)
        references games(tournament_id, id)
        on delete set null (featured_game_id)
        deferrable initially deferred;

create unique index games_one_per_tournament_slot
    on games (tournament_id, tournament_slot_id)
    where tournament_slot_id is not null;
create unique index games_arena_ordinal_unique
    on games (tournament_id, arena_ordinal)
    where arena_ordinal is not null;
create index games_tournament on games (tournament_id);
create index games_white on games (finished, white_id, nanoid);
create index games_black on games (finished, black_id, nanoid);
create index games_arena_move_due_at
    on games (arena_move_due_at, id)
    where arena_move_due_at is not null and not finished;

create table tournament_swiss_rounds (
    tournament_id uuid not null references tournaments(id) on delete cascade,
    round_id int8 not null,
    pairings jsonb not null,
    accepted_at timestamptz not null,
    primary key (tournament_id, round_id),
    constraint tournament_swiss_rounds_round_id_check check (round_id >= 0),
    constraint tournament_swiss_rounds_pairings_check
        check (
            (
                jsonb_typeof(pairings) = 'object'
                and pairings - array['games', 'byes', 'accepted_ratings']::text[] = '{}'::jsonb
                and jsonb_typeof(pairings -> 'games') = 'array'
                and jsonb_typeof(pairings -> 'byes') = 'array'
                and jsonb_typeof(pairings -> 'accepted_ratings') = 'array'
            ) is true
        )
);

create table tournament_elimination_nodes (
    tournament_id uuid not null references tournaments(id) on delete cascade,
    node_id int8 not null,
    fact jsonb not null,
    primary key (tournament_id, node_id),
    constraint tournament_elimination_nodes_node_id_check check (node_id >= 0),
    constraint tournament_elimination_nodes_fact_check
        check (
            (
                jsonb_typeof(fact) = 'object'
                and fact - array['node', 'state']::text[] = '{}'::jsonb
                and jsonb_typeof(fact -> 'node') = 'number'
                and fact ->> 'node' ~ '^(0|[1-9][0-9]*)$'
                and (fact ->> 'node')::numeric = node_id
                and fact ? 'state'
            ) is true
        )
);

create table schedule_offers (
    id uuid primary key not null default gen_random_uuid(),
    tournament_id uuid not null,
    tournament_slot_id uuid not null,
    proposer_id uuid not null references users(id),
    candidate_times timestamptz[] not null,
    status text not null default 'pending',
    selected_time timestamptz,
    created_at timestamptz not null default now(),
    resolved_at timestamptz,
    resolved_by uuid references users(id),
    notified boolean not null default false,
    constraint schedule_offers_candidates_check check (
        array_ndims(candidate_times) = 1
        and array_lower(candidate_times, 1) = 1
        and cardinality(candidate_times) between 1 and 3
        and array_position(candidate_times, null) is null
        and (cardinality(candidate_times) < 2 or candidate_times[1] <> candidate_times[2])
        and (cardinality(candidate_times) < 3 or candidate_times[1] <> candidate_times[3])
        and (cardinality(candidate_times) < 3 or candidate_times[2] <> candidate_times[3])
    ),
    constraint schedule_offers_status_check check (
        status in ('pending', 'accepted', 'declined', 'withdrawn', 'superseded', 'cancelled')
    ),
    constraint schedule_offers_resolution_check check (
        (
            status = 'pending'
            and selected_time is null
            and resolved_at is null
            and resolved_by is null
            and not notified
        )
        or
        (
            status = 'accepted'
            and selected_time = any(candidate_times)
            and resolved_at is not null
            and resolved_by is not null
        )
        or
        (
            status in ('declined', 'withdrawn', 'superseded', 'cancelled')
            and selected_time is null
            and resolved_at is not null
            and not notified
        )
    ),
    constraint schedule_offers_resolution_time_check check (
        resolved_at is null or resolved_at >= created_at
    ),
    constraint schedule_offers_notification_check check (
        not notified or status = 'accepted'
    ),
    constraint schedule_offers_tournament_slot_fkey foreign key (tournament_id, tournament_slot_id)
        references tournament_slots(tournament_id, id)
        on delete cascade,
    constraint schedule_offers_proposer_membership foreign key (tournament_id, proposer_id)
        references tournaments_users(tournament_id, user_id)
        deferrable initially deferred
);

create function enforce_schedule_offer_slot_proposer()
returns trigger
language plpgsql
as $$
declare
    slot_white_id uuid;
    slot_black_id uuid;
begin
    select white_id, black_id
    into slot_white_id, slot_black_id
    from tournament_slots
    where tournament_id = new.tournament_id and id = new.tournament_slot_id;

    if found and new.proposer_id not in (slot_white_id, slot_black_id) then
        raise exception using
            errcode = '23514',
            constraint = 'schedule_offers_slot_proposer_check',
            message = 'schedule offer proposer must be a Slot entrant';
    end if;

    return new;
end;
$$;

create trigger schedule_offers_slot_proposer_check
before insert or update of tournament_id, tournament_slot_id, proposer_id
on schedule_offers
for each row execute function enforce_schedule_offer_slot_proposer();

create unique index schedule_offers_one_pending_per_slot
    on schedule_offers (tournament_id, tournament_slot_id)
    where status = 'pending';
create index schedule_offers_slot_history
    on schedule_offers (tournament_id, tournament_slot_id, created_at, id);
create index schedule_offers_acceptance_notifications
    on schedule_offers (proposer_id, resolved_at, id)
    where status = 'accepted' and not notified;
create index tournament_slots_scheduled_at
    on tournament_slots (scheduled_at, tournament_id, id)
    where scheduled_at is not null;
create index tournament_slots_deadline_at
    on tournament_slots (deadline_at, tournament_id, id)
    where deadline_at is not null and resolution is null;

create table tournament_final_outcomes (
    tournament_id uuid primary key references tournaments(id) on delete cascade,
    standings jsonb not null,
    arena_ratings jsonb,
    constraint tournament_final_outcomes_arena_ratings_check
        check (arena_ratings is null or jsonb_typeof(arena_ratings) = 'array'),
    constraint tournament_final_outcomes_standings_check
        check (
            (
                jsonb_typeof(standings) = 'object'
                and standings - array['groups']::text[] = '{}'::jsonb
                and jsonb_typeof(standings -> 'groups') = 'array'
            ) is true
        )
);

-- Completion only references its own Game: no tournament-row lock is needed
-- while unrelated games complete or the finalizer freezes its snapshot.
create table arena_game_results (
    game_id uuid primary key,
    tournament_id uuid not null,
    white_points int8 not null check (white_points between 0 and 4294967295),
    black_points int8 not null check (black_points between 0 and 4294967295),
    white_doubled boolean not null,
    black_doubled boolean not null,
    unique (tournament_id, game_id),
    foreign key (tournament_id, game_id) references games(tournament_id, id)
);

create table tournament_final_arena_results (
    tournament_id uuid not null references tournament_final_outcomes(tournament_id) on delete cascade,
    game_id uuid not null,
    primary key (tournament_id, game_id),
    foreign key (tournament_id, game_id) references arena_game_results(tournament_id, game_id)
);
