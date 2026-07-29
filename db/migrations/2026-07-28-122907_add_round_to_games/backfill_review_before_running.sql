-- NOT run by diesel. Diesel only executes up.sql and down.sql; this file is
-- here to be read, checked against your data, and run by hand.
--
-- Why it is needed: replay treats a tournament game with a NULL round as a
-- hard error, because a round cannot be inferred once players have met more
-- than once. Every game created before the `round` column existed has NULL.
--
-- This applies to EVERY mode, round robin included. There is no bucketing
-- fallback — an earlier version of this note claimed one, and it was wrong.
--
-- Finished tournaments need it too: `standings()` replays them from scratch on
-- every request, so a finished tournament whose games have no round will fail
-- to render rather than quietly losing its progressive score.
--
-- What it does: walks each affected tournament's games oldest first, and
-- starts a new round whenever it meets a player who has already played in the
-- round being built. Double-Swiss pairs are two rows for one pairing, so
-- consecutive rows for the same unordered pair are treated as one unit.
--
-- KNOWN LIMITATION, read before running: if the same two players are paired
-- again in the very next round, their rows look like two units of the same
-- pair back to back and get collapsed into one round. Dutch and Burstein never
-- pair a rematch, so they are safe. Double-Swiss can, though it is rare, and so
-- can a multi-cycle round robin at a cycle boundary. Step 1 finds them; fix
-- those tournaments by hand.
--
-- The three steps below are in run order, so the file is safe to run
-- top-to-bottom — but stop and read the output of step 1 before continuing.


-- STEP 1. Any row this returns is a tournament where the collapse described
-- above can happen, and which should be checked by hand. Dutch and Burstein
-- cannot rematch, so they are excluded.

select g.tournament_id, t.mode, count(*) as repeat_pairings
from games g
join games later
  on later.tournament_id = g.tournament_id
  and least(later.white_id, later.black_id) = least(g.white_id, g.black_id)
  and greatest(later.white_id, later.black_id) = greatest(g.white_id, g.black_id)
  and later.created_at > g.created_at
join tournaments t on t.id = g.tournament_id
where g.round is null
  and t.mode in (
    'DoubleSwiss',
    'SingleElimination', 'DoubleElimination',
    'DoubleRoundRobin', 'QuadrupleRoundRobin', 'SextupleRoundRobin'
  )
group by g.tournament_id, t.mode
having count(*) > 2;


-- STEP 2. The backfill itself.

do $$
declare
    tournament record;
    game record;
    current_round int;
    current_players uuid[];
    previous_pair uuid[];
    this_pair uuid[];
begin
    for tournament in
        select t.id
        from tournaments t
        where t.mode in (
            'DoubleSwiss', 'DutchSwiss', 'BursteinSwiss',
            'SingleElimination', 'DoubleElimination',
            'SingleRoundRobin', 'DoubleRoundRobin',
            'QuadrupleRoundRobin', 'SextupleRoundRobin'
        )
        order by t.created_at
    loop
        current_round := 0;
        current_players := array[]::uuid[];
        previous_pair := null;

        for game in
            select g.id, g.white_id, g.black_id
            from games g
            where g.tournament_id = tournament.id and g.round is null
            order by g.created_at, g.id
        loop
            this_pair := array[
                least(game.white_id, game.black_id),
                greatest(game.white_id, game.black_id)
            ];

            -- The second row of a two-game match belongs with the first.
            if previous_pair is distinct from this_pair then
                if game.white_id = any(current_players)
                    or game.black_id = any(current_players)
                then
                    current_round := current_round + 1;
                    current_players := array[]::uuid[];
                end if;
                current_players := current_players
                    || game.white_id || game.black_id;
            end if;

            if current_round = 0 then
                current_round := 1;
            end if;

            update games set round = current_round where id = game.id;
            previous_pair := this_pair;
        end loop;
    end loop;
end $$;

-- STEP 3. Verification: this must return no rows.

select id, tournament_id from games
where tournament_id is not null and round is null;
