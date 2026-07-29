-- A player may pause and resume any number of times, so these cannot live as
-- columns on tournaments_users the way joined_at does. The arena is replayed
-- from a timeline, and each of these is one more event on it.
create table tournament_arena_events (
    id serial primary key,
    tournament_id uuid not null references tournaments(id) on delete cascade,
    user_id uuid not null references users(id) on delete cascade,
    kind text not null,
    at timestamptz not null
);

create index tournament_arena_events_replay on tournament_arena_events (tournament_id, at, id);

-- Events are stamped from the arena clock, so one player cannot legitimately
-- have two of the same kind at the same instant. Writes are serialized by the
-- tournament row lock; this is the backstop, because a duplicate is not a
-- recoverable error — replay would apply it twice, the engine would refuse,
-- and every later read of the arena would fail with it.
create unique index tournament_arena_events_once
    on tournament_arena_events (tournament_id, user_id, kind, at);
