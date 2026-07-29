use crate::{
    db_error::DbError,
    models::{
        tournament_engine::{Field, POINT_SCALE},
        Game,
        NewGame,
        Rating,
        Tournament,
        TournamentArenaEvent,
        TournamentUser,
    },
    schema::{
        games,
        tournaments_users::{self, user_id},
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::Color as HiveColor;
use shared_types::{
    ArenaEventKind,
    Conclusion,
    GameSpeed,
    PlayerScores,
    PlayerStanding,
    PointSystemDetails,
    Standings,
    Tiebreaker,
    TournamentGameResult,
};
use std::{collections::HashMap, str::FromStr};
use tournamint::{
    arena::{ArenaConfig, ArenaGameId, ArenaGameOutcome, ArenaTime, ArenaTournament},
    MatchScore,
    Rating as EngineRating,
};
use uuid::Uuid;

/// What an arena scores by convention, before the tournament's own overrides.
/// There are no byes in an arena — nobody sits out a round, because there are
/// no rounds — so both bye values are nothing.
fn arena_default_points() -> PointSystemDetails {
    PointSystemDetails {
        win: Some(1.0),
        draw: Some(0.5),
        loss: Some(0.0),
        forfeit_loss: Some(0.0),
        zero_point_bye: Some(0.0),
        pairing_allocated_bye: Some(0.0),
    }
}

/// One thing that happened, at the arena-clock instant it happened.
///
/// An arena cannot be replayed the way a Swiss can, because who gets paired
/// depends on *when* each earlier game ended. So the timeline is rebuilt from
/// the timestamps we already store and replayed in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Join { player: usize },
    Finish { game: usize },
    Break { player: usize, kind: ArenaEventKind },
    Pair,
}

/// Ordering for events that share an instant. Everything that changes who is
/// pairable has to land before the pairing at that moment: a join and a
/// finished game put players *into* the pool, a pause or withdrawal takes them
/// out, and the pairing then sees the pool as it really was.
fn event_priority(event: &Event) -> u8 {
    match event {
        Event::Join { .. } => 0,
        Event::Finish { .. } => 1,
        Event::Break { .. } => 2,
        Event::Pair => 3,
    }
}

pub(crate) struct ArenaReplay {
    pub(crate) arena: ArenaTournament,
    pub(crate) field: Field,
    pub(crate) now: ArenaTime,
}

fn arena_outcome(game: &Game) -> Result<ArenaGameOutcome, DbError> {
    let result = TournamentGameResult::from_str(&game.tournament_game_result).map_err(|_| {
        DbError::InvalidAction {
            info: format!("{} is not a valid result", game.tournament_game_result),
        }
    })?;
    // A withdrawal forfeit is a game nobody played.
    let forfeit = matches!(
        Conclusion::from_str(&game.conclusion),
        Ok(Conclusion::Forfeit) | Ok(Conclusion::Withdrawal)
    );
    let white_score = match result {
        TournamentGameResult::Winner(HiveColor::White) => MatchScore::Win,
        TournamentGameResult::Winner(HiveColor::Black) => MatchScore::Loss,
        TournamentGameResult::Draw | TournamentGameResult::DoubeForfeit => MatchScore::Draw,
        TournamentGameResult::Unknown => {
            return Err(DbError::InvalidAction {
                info: String::from("cannot score an arena game with an unknown result"),
            })
        }
    };
    Ok(ArenaGameOutcome {
        white_score,
        game_was_played: !forfeit && result != TournamentGameResult::DoubeForfeit,
        white_berserked: game.white_berserked,
        black_berserked: game.black_berserked,
        // Drives the quick-draw and long-game rules that stop two players
        // farming each other with instant agreed draws.
        turns: Some(game.turn.max(0) as usize),
    })
}

impl Tournament {
    fn arena_duration(&self) -> Result<ArenaTime, DbError> {
        let seconds = self
            .arena_duration_seconds
            .ok_or_else(|| DbError::InvalidAction {
                info: String::from("arena tournament has no duration"),
            })?;
        Ok(ArenaTime::from_seconds(seconds.max(0) as u64))
    }

    fn started_at_or_err(&self) -> Result<DateTime<Utc>, DbError> {
        self.started_at.ok_or_else(|| DbError::InvalidAction {
            info: String::from("arena has not started"),
        })
    }

    /// Wall-clock instant to arena time. Anything before the start clamps to
    /// zero; the engine refuses a clock that goes backwards.
    fn arena_time(&self, at: DateTime<Utc>) -> Result<ArenaTime, DbError> {
        let started = self.started_at_or_err()?;
        let millis = at.signed_duration_since(started).num_milliseconds();
        Ok(ArenaTime::from_millis(millis.max(0) as u64))
    }

    pub(crate) async fn replay_arena(
        &self,
        now: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<ArenaReplay, DbError> {
        let duration = self.arena_duration()?;
        let players = self.seeded_players(conn).await?;
        let field = Field::build_in_join_order(players.clone())?;

        let games: Vec<Game> = games::table
            .filter(games::tournament_id.eq(Some(self.id)))
            .order((games::created_at.asc(), games::id.asc()))
            .get_results(conn)
            .await?;

        // Join times come from tournaments_users; a pairing moment is a game's
        // created_at, and a result is its updated_at.
        let joined_at: HashMap<Uuid, DateTime<Utc>> = tournaments_users::table
            .filter(tournaments_users::tournament_id.eq(self.id))
            .select((user_id, tournaments_users::joined_at))
            .get_results::<(Uuid, Option<DateTime<Utc>>)>(conn)
            .await?
            .into_iter()
            .filter_map(|(user, at)| at.map(|at| (user, at)))
            .collect();

        let seat_of: HashMap<Uuid, usize> = players
            .iter()
            .enumerate()
            .map(|(index, player)| (player.user_id, index))
            .collect();

        let mut timeline: Vec<(ArenaTime, u8, Event)> = Vec::new();
        let push = |at: ArenaTime, event: Event, into: &mut Vec<(ArenaTime, u8, Event)>| {
            into.push((at, event_priority(&event), event));
        };

        for (index, player) in players.iter().enumerate() {
            let at = joined_at
                .get(&player.user_id)
                .copied()
                .unwrap_or_else(|| self.started_at.unwrap_or(now));
            push(
                self.arena_time(at)?,
                Event::Join { player: index },
                &mut timeline,
            );
        }
        for (index, game) in games.iter().enumerate() {
            push(
                self.arena_time(game.created_at)?,
                Event::Pair,
                &mut timeline,
            );
            if game.finished {
                // `updated_at` moves whenever anything writes the row again —
                // an organizer re-adjudicating, a reinstatement — which would
                // silently reorder an event the timeline has already replayed.
                let at = game.finished_at.ok_or_else(|| DbError::InvalidAction {
                    info: format!("arena game {} finished without a finish time", game.id),
                })?;
                push(
                    self.arena_time(at)?,
                    Event::Finish { game: index },
                    &mut timeline,
                );
            }
        }
        for event in TournamentArenaEvent::for_tournament(self.id, conn).await? {
            let player = *seat_of
                .get(&event.user_id)
                .ok_or_else(|| DbError::InvalidAction {
                    info: format!(
                        "{} took a break from an arena they never joined",
                        event.user_id
                    ),
                })?;
            push(
                self.arena_time(event.at)?,
                Event::Break {
                    player,
                    kind: event.kind()?,
                },
                &mut timeline,
            );
        }
        timeline.sort_by_key(|(at, priority, _)| (*at, *priority));

        let mut arena = ArenaTournament::new(ArenaConfig::full_scoring(
            duration,
            self.point_system_from(arena_default_points()),
        ))
        .map_err(|error| DbError::InvalidAction {
            info: format!("could not build the arena: {error}"),
        })?;

        // Stored id -> the arena's own id. Persisted on the row precisely so a
        // result never has to be matched back by guessing.
        let mut arena_ids: HashMap<Uuid, ArenaGameId> = HashMap::new();
        let mut paired_at: Option<ArenaTime> = None;
        // Players who left of their own accord; withdrawing them a second time
        // for being deleted would be refused.
        let mut withdrawn: std::collections::HashSet<Uuid> = std::collections::HashSet::new();

        for (at, _, event) in timeline {
            match event {
                Event::Join { player } => {
                    let seeded = &players[player];
                    let rating = seeded
                        .rating
                        .map(|rating| EngineRating::new(rating.max(0.0).round() as usize));
                    arena
                        .join(at, field.player_id(seeded.user_id)?, rating)
                        .map_err(|error| DbError::InvalidAction {
                            info: format!("could not replay a join: {error}"),
                        })?;
                }
                Event::Break { player, kind } => {
                    let seat = field.player_id(players[player].user_id)?;
                    let applied = match kind {
                        ArenaEventKind::Pause => arena.pause(at, seat),
                        ArenaEventKind::Resume => arena.resume(at, seat),
                        ArenaEventKind::Withdraw => arena.withdraw(at, seat),
                    };
                    applied.map_err(|error| DbError::InvalidAction {
                        info: format!("could not replay a {kind}: {error}"),
                    })?;
                    if kind == ArenaEventKind::Withdraw {
                        withdrawn.insert(players[player].user_id);
                    }
                }
                Event::Pair => {
                    // Several games are created by one `pair_waiting` call, so
                    // only pair once per distinct instant.
                    if paired_at == Some(at) {
                        continue;
                    }
                    paired_at = Some(at);
                    let paired =
                        arena
                            .pair_waiting(at)
                            .map_err(|error| DbError::InvalidAction {
                                info: format!("could not replay a pairing: {error}"),
                            })?;
                    let created: Vec<&Game> = games
                        .iter()
                        .filter(|game| self.arena_time(game.created_at).ok() == Some(at))
                        .collect();
                    if paired.len() != created.len() {
                        return Err(DbError::InvalidAction {
                            info: format!(
                                "arena replay diverged: engine paired {} games, {} are stored",
                                paired.len(),
                                created.len()
                            ),
                        });
                    }
                    for game in paired {
                        // Matched on the id the engine itself handed out and we
                        // stored, not on the players: two entrants can have
                        // several games together over a long arena, so a
                        // (white, black) lookup is not unique.
                        let index = game.id.index() as i32;
                        let stored = created
                            .iter()
                            .find(|row| row.arena_game_id == Some(index))
                            .ok_or_else(|| DbError::InvalidAction {
                                info: format!(
                                    "arena replay diverged: no stored game carries arena id {index}"
                                ),
                            })?;
                        // The engine is the authority on who plays whom; if the
                        // stored row disagrees, the replay has drifted and every
                        // later result would attach to the wrong game.
                        if stored.white_id != field.user_id(game.white)?
                            || stored.black_id != field.user_id(game.black)?
                        {
                            return Err(DbError::InvalidAction {
                                info: format!(
                                    "arena replay diverged: arena game {index} paired different players than were stored"
                                ),
                            });
                        }
                        arena_ids.insert(stored.id, game.id);
                    }
                }
                Event::Finish { game } => {
                    let stored = &games[game];
                    let id = arena_ids.get(&stored.id).copied().ok_or_else(|| {
                        DbError::InvalidAction {
                            info: String::from("a finished arena game was never paired"),
                        }
                    })?;
                    arena
                        .record_result(at, id, arena_outcome(stored)?)
                        .map_err(|error| DbError::InvalidAction {
                            info: format!("could not replay an arena result: {error}"),
                        })?;
                }
            }
        }

        let now = self.arena_time(now)?.max(arena.now());

        // A deleted account keeps whatever it scored, but must never be paired
        // again. This mirrors what the Swiss path does with `withdraw_player`,
        // and is applied after the history so past results still stand.
        for player in players.iter().filter(|player| player.withdrawn) {
            if withdrawn.contains(&player.user_id) {
                continue;
            }
            arena
                .withdraw(now, field.player_id(player.user_id)?)
                .map_err(|error| DbError::InvalidAction {
                    info: format!("could not withdraw a deleted player: {error}"),
                })?;
        }

        Ok(ArenaReplay { arena, field, now })
    }

    /// Lets a player into an arena that is already running. Their seed is
    /// handed out here rather than at start, because the engine indexes players
    /// by arrival.
    pub async fn join_arena(
        &self,
        user: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentUser, DbError> {
        // Seats are counted and then handed out, so two joins racing would
        // otherwise both take the same one and surface as a raw unique
        // violation. The lock also keeps a join from landing between a
        // pairing's replay and its writes.
        let locked = self.lock_row(conn).await?;
        locked.ensure_inprogress()?;
        if !locked.mode()?.is_arena() {
            return Err(DbError::InvalidAction {
                info: String::from("only an arena can be joined after it starts"),
            });
        }

        // Already being in the arena is checked first, so a member who clicks
        // join twice is told that rather than that the arena is full.
        if tournaments_users::table
            .find((self.id, *user))
            .count()
            .get_result::<i64>(conn)
            .await?
            > 0
        {
            return Err(DbError::InvalidAction {
                info: String::from("already in this arena"),
            });
        }

        // Joining late still has to respect what gates joining at all: an arena
        // can fill up, and it can be invite only.
        locked.ensure_not_invite_only(user, conn).await?;
        locked.ensure_not_full(conn).await?;

        // Seats are handed out by arrival, because the engine indexes its
        // player table directly and rejects a non-sequential id.
        let existing: i64 = tournaments_users::table
            .filter(tournaments_users::tournament_id.eq(self.id))
            .count()
            .get_result(conn)
            .await?;
        let speed = GameSpeed::from_base_increment(self.time_base, self.time_increment);
        let rating = Rating::for_uuid(user, &speed, conn)
            .await
            .ok()
            .map(|rating| rating.rating);

        let entrant = TournamentUser::new_arena_entrant(self.id, *user, existing as i32, rating);
        entrant.insert(conn).await?;
        Ok(entrant)
    }

    /// Pairs everyone waiting, right now. This is the arena's whole engine
    /// loop: there are no rounds, so a player becomes pairable again the
    /// instant their own game is recorded.
    pub async fn pair_arena(
        &self,
        now: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        // The pool is read by replaying, and the games are written afterwards.
        // A join or a pause landing in between would make the stored pairing
        // disagree with what any later replay derives, which is fatal and
        // unrecoverable for the arena.
        let locked = self.lock_row(conn).await?;
        let ArenaReplay {
            mut arena,
            field,
            now: arena_now,
        } = locked.replay_arena(now, conn).await?;

        if arena.is_expired(arena_now) {
            return Ok(Vec::new());
        }

        let paired = arena
            .pair_waiting(arena_now)
            .map_err(|error| DbError::InvalidAction {
                info: format!("could not pair the arena: {error}"),
            })?;

        // Every game from one `pair_waiting` call must carry the *same*
        // created_at, because replay reads that timestamp back as "the arena
        // paired at this instant". Per-row `Utc::now()` values could straddle a
        // millisecond and replay as two pairing moments, pairing more games
        // than were stored.
        let paired_instant =
            locked.started_at_or_err()? + chrono::Duration::milliseconds(arena_now.millis() as i64);

        let mut games = Vec::new();
        for game in paired {
            let white = field.user_id(game.white)?;
            let black = field.user_id(game.black)?;
            let mut new_game = NewGame::new_from_tournament(white, black, &locked, 0);
            // An arena has no rounds; the pairing index identifies a game here.
            new_game.round = None;
            new_game.arena_game_id = Some(game.id.index() as i32);
            new_game.created_at = paired_instant;
            games.push(Game::create(new_game, conn).await?);
        }
        Ok(games)
    }

    /// Steps a player out of the pairing pool. If they are mid-game the break
    /// is remembered and starts when that game ends — the game still counts.
    pub async fn pause_in_arena(
        &self,
        user: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentArenaEvent, DbError> {
        self.record_break(user, ArenaEventKind::Pause, conn).await
    }

    /// Puts a paused player back in the pool, or calls off a break that was
    /// requested mid-game and has not started yet.
    pub async fn resume_in_arena(
        &self,
        user: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentArenaEvent, DbError> {
        self.record_break(user, ArenaEventKind::Resume, conn).await
    }

    /// Removes a player for good. What they have already scored stands and
    /// keeps counting toward everyone else's standings.
    pub async fn withdraw_from_arena(
        &self,
        user: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentArenaEvent, DbError> {
        self.record_break(user, ArenaEventKind::Withdraw, conn)
            .await
    }

    /// Applies the change to a replayed arena first and only writes it if the
    /// engine accepts it. Persisting an event the engine would reject — a
    /// second pause, say — would make every later replay fail, taking the
    /// standings down with it.
    async fn record_break(
        &self,
        user: &Uuid,
        kind: ArenaEventKind,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentArenaEvent, DbError> {
        // Validating against a replay and then writing is only sound if nothing
        // else can write in between. Without this a double-clicked pause writes
        // two rows, and every later replay then pauses twice and fails — which
        // kills standings, pairing and progress for the arena with no way back.
        let locked = self.lock_row(conn).await?;
        locked.ensure_inprogress()?;
        if !locked.mode()?.is_arena() {
            return Err(DbError::InvalidAction {
                info: String::from("only an arena can be paused or left mid-tournament"),
            });
        }

        let now = Utc::now();
        let ArenaReplay {
            mut arena,
            field,
            now: arena_now,
        } = locked.replay_arena(now, conn).await?;
        let seat = field.player_id(*user)?;

        let applied = match kind {
            ArenaEventKind::Pause => arena.pause(arena_now, seat),
            ArenaEventKind::Resume => arena.resume(arena_now, seat),
            ArenaEventKind::Withdraw => arena.withdraw(arena_now, seat),
        };
        applied.map_err(|error| DbError::InvalidAction {
            info: format!("cannot {kind} in this arena: {error}"),
        })?;

        // Stamped from the arena clock, not the wall clock, so the event lands
        // exactly where the replay expects it even if the two have drifted.
        let at =
            locked.started_at_or_err()? + chrono::Duration::milliseconds(arena_now.millis() as i64);
        TournamentArenaEvent::record(locked.id, *user, kind, at, conn).await
    }

    pub async fn arena_is_over(
        &self,
        now: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        let replay = self.replay_arena(now, conn).await?;
        Ok(replay.arena.is_finished(replay.now))
    }

    /// An arena's table is its own: no Buchholz, because with no fixed round
    /// count "who you played" says nothing comparable. Points, then wins, then
    /// fewest games — reaching a score in fewer games is the better run.
    pub(crate) async fn arena_standings(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<Standings, DbError> {
        let ArenaReplay { arena, field, .. } = self.replay_arena(Utc::now(), conn).await?;
        let tiebreakers = arena_tiebreakers(self);
        let metrics = arena.standing_metrics();

        let mut groups: Vec<Vec<PlayerStanding>> = Vec::new();
        let mut position = 1;
        for group in arena.standings() {
            let at = position;
            position += group.len() as u32;
            let mut row = Vec::with_capacity(group.len());
            for player in group {
                let metric = metrics
                    .iter()
                    .find(|metric| metric.player_id == player)
                    .ok_or_else(|| DbError::InvalidAction {
                        info: String::from("a ranked arena player has no metrics"),
                    })?;
                let mut scores = PlayerScores::new();
                for tiebreaker in &tiebreakers {
                    let value = match tiebreaker {
                        Tiebreaker::RawPoints => metric.points as f32 / POINT_SCALE as f32,
                        Tiebreaker::Wins => metric.wins as f32,
                        Tiebreaker::Draws => metric.draws as f32,
                        Tiebreaker::Losses => metric.losses as f32,
                        Tiebreaker::GamesPlayed => metric.games_played as f32,
                        Tiebreaker::CurrentStreak => metric.current_streak as f32,
                        Tiebreaker::BestStreak => metric.best_streak as f32,
                        Tiebreaker::Berserks => metric.berserks as f32,
                        _ => continue,
                    };
                    scores.insert(*tiebreaker, value);
                }
                row.push(PlayerStanding {
                    player: field.user_id(player)?,
                    position: at,
                    games_played: metric.games_played as i32,
                    scores,
                });
            }
            groups.push(row);
        }

        Ok(Standings {
            tiebreakers,
            groups,
        })
    }
}

/// The arena's own ranking key leads, then whatever else the organizer asked
/// to display. Reordering is not offered: points, wins, fewest games is the
/// engine's own order and the standings groups come back already applied.
fn arena_tiebreakers(tournament: &Tournament) -> Vec<Tiebreaker> {
    let mut order = vec![
        Tiebreaker::RawPoints,
        Tiebreaker::Wins,
        Tiebreaker::GamesPlayed,
    ];
    for stored in tournament.tiebreaker.iter().flatten() {
        if let Ok(tiebreaker) = Tiebreaker::from_str(stored) {
            if !order.contains(&tiebreaker) {
                order.push(tiebreaker);
            }
        }
    }
    for extra in [
        Tiebreaker::Draws,
        Tiebreaker::Losses,
        Tiebreaker::BestStreak,
        Tiebreaker::CurrentStreak,
        Tiebreaker::Berserks,
    ] {
        if !order.contains(&extra) {
            order.push(extra);
        }
    }
    order
}
