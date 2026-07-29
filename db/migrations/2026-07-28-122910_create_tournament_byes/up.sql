create table tournament_byes (
    tournament_id uuid not null references tournaments(id) on delete cascade,
    round int4 not null,
    user_id uuid not null references users(id) on delete cascade,
    -- What the bye is worth. A pairing-allocated bye is the odd player out and
    -- pays a full point; a zero-point bye is somebody sitting the round out by
    -- request and pays `points_zero_point_bye`. Replay has to know which,
    -- because the engine scores them differently and neither can be inferred
    -- from the absence of a game.
    kind text not null default 'PairingAllocated',
    primary key (tournament_id, round, user_id)
);
