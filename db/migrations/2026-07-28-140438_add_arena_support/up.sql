-- Arenas run against a wall clock rather than a round count.
alter table tournaments add column arena_duration_seconds int4;

-- Arena players join while the clock runs, and the pairing engine indexes
-- players by arrival order, so the join instant is load-bearing.
alter table tournaments_users add column joined_at timestamptz;

-- Berserk is declared per player and halves only that player's clock, so it
-- cannot live on the shared time_base/time_increment.
alter table games add column white_berserked boolean not null default false;
alter table games add column black_berserked boolean not null default false;

-- The arena assigns each game an id by pairing order. Storing it means a
-- result can be attached to the right game without re-deriving that order.
alter table games add column arena_game_id int4;

-- The arena hands out ids by pairing order, so two rows sharing one would make
-- the replay ambiguous about which game a result belongs to. Pairing is
-- serialized by the tournament row lock; this makes a duplicate impossible
-- even if something ever pairs outside it.
create unique index games_arena_game_id
    on games (tournament_id, arena_game_id)
    where arena_game_id is not null;

-- When a game finished, as opposed to when its row was last written. The arena
-- timeline is rebuilt from stored timestamps and replayed in order, so timing a
-- result from updated_at would let any later write to the row — an organizer
-- re-adjudicating, a reinstatement un-forfeiting — silently move an event that
-- has already happened and diverge the replay.
alter table games add column finished_at timestamptz;
