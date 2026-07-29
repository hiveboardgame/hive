use crate::{
    db_error::DbError,
    models::{Game, NewGame, Rating, Schedule, Tournament, TournamentArenaEvent, TournamentBye},
    schema::{
        games::{self, created_at, id as game_id_column, round as round_column, tournament_id},
        tournaments::{self, rounds as rounds_column, updated_at},
        tournaments_users::{self, rating as rating_column, seed as seed_column, user_id},
        users,
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, PgSortExpressionMethods};
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::Color as HiveColor;
use shared_types::{
    ArenaEventKind,
    ByeKind as StoredByeKind,
    Conclusion,
    GameSpeed,
    PointSystemDetails,
    ScoringMode,
    TournamentGameResult,
    TournamentMode,
    TournamentStatus,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    str::FromStr,
};
use tournamint::{
    elimination::{
        DoubleEliminationBracket,
        EliminationSubmitError,
        NextRoundError,
        SingleEliminationBracket,
    },
    round_robin::round_robin_schedule,
    swiss::{pair_next_round, SubmitError, SwissTournament, TiebreakBasis},
    ByeAssignment,
    ByeKind,
    Color,
    GameOutcome,
    MatchScore,
    Pairing,
    PlayerId,
    PointSystem,
    PointSystemInput,
    Rating as EngineRating,
    Roster,
    RoundIndex,
    Score,
    SwissSystem,
};
use uuid::Uuid;

/// Scores are integers in the engine, so a draw is representable only by
/// scaling. Everything read back out is divided by this again to get the
/// familiar 1 / 0.5 / 0. Shared with the validation in `PointSystemDetails`,
/// which rejects anything finer than this can store.
pub(crate) const POINT_SCALE: usize = PointSystemDetails::SCALE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EngineFormat {
    Swiss(SwissSystem),
    RoundRobin { repeats: usize },
    SingleElimination { third_place: bool },
    DoubleElimination,
}

impl EngineFormat {
    fn for_tournament(tournament: &Tournament) -> Result<Self, DbError> {
        let mode = tournament.mode()?;
        if let Some(repeats) = mode.round_robin_repeats() {
            return Ok(Self::RoundRobin { repeats });
        }
        Ok(match mode {
            TournamentMode::DutchSwiss => Self::Swiss(SwissSystem::Dutch),
            TournamentMode::BursteinSwiss => Self::Swiss(SwissSystem::Burstein),
            TournamentMode::DoubleSwiss => Self::Swiss(SwissSystem::DoubleSwiss),
            TournamentMode::SingleElimination => Self::SingleElimination {
                third_place: tournament.third_place_match,
            },
            TournamentMode::DoubleElimination => Self::DoubleElimination,
            other => {
                return Err(DbError::InvalidAction {
                    info: format!("{other} has no pairing engine"),
                })
            }
        })
    }

    /// Round robin's rounds are independent of one another, so a round can be
    /// replayed with only the games that have actually finished. Swiss and the
    /// brackets are sequential: a round must be declared whole.
    fn is_sequential(&self) -> bool {
        !matches!(self, Self::RoundRobin { .. })
    }

    fn is_bracket(&self) -> bool {
        matches!(
            self,
            Self::SingleElimination { .. } | Self::DoubleElimination
        )
    }

    fn plays_two_game_matches(&self) -> bool {
        matches!(
            self,
            Self::Swiss(SwissSystem::DoubleSwiss)
                | Self::SingleElimination { .. }
                | Self::DoubleElimination
        )
    }

    /// What this format scores by convention, before the tournament's own
    /// overrides are applied.
    fn default_points(&self) -> PointSystemDetails {
        // A round-robin bye is a rest, not a result — everyone gets one and it
        // earns nothing. A Swiss bye is a full point, per FIDE, which is what
        // the retired SwissByePlayer games used to hand out.
        let pairing_allocated_bye = match self {
            Self::RoundRobin { .. } => 0.0,
            _ => 1.0,
        };
        PointSystemDetails {
            win: Some(1.0),
            draw: Some(0.5),
            loss: Some(0.0),
            forfeit_loss: Some(0.0),
            zero_point_bye: Some(0.0),
            pairing_allocated_bye: Some(pairing_allocated_bye),
        }
    }
}

/// Human units to the engine's integers. A draw has to survive the trip, so
/// everything is doubled — which is why half a point is the finest step a
/// tournament can configure.
fn scaled(value: f64) -> Score {
    Score::new((value * POINT_SCALE as f64).round().max(0.0) as usize)
}

impl Tournament {
    /// The tournament's own point system, falling back per value to whatever
    /// its format normally does.
    pub(crate) fn point_system(&self, format: EngineFormat) -> PointSystem {
        self.point_system_from(format.default_points())
    }

    /// As above, for a format that has no `EngineFormat` of its own — an arena
    /// is scored but not paired by round, so it supplies its own defaults.
    pub(crate) fn point_system_from(&self, defaults: PointSystemDetails) -> PointSystem {
        let value =
            |set: Option<f64>, fallback: Option<f64>| scaled(set.or(fallback).unwrap_or(0.0));
        PointSystem::new(PointSystemInput {
            win: value(self.points_win, defaults.win),
            draw: value(self.points_draw, defaults.draw),
            loss: value(self.points_loss, defaults.loss),
            zero_point_bye: value(self.points_zero_point_bye, defaults.zero_point_bye),
            forfeit_loss: value(self.points_forfeit_loss, defaults.forfeit_loss),
            pairing_allocated_bye: value(
                self.points_pairing_allocated_bye,
                defaults.pairing_allocated_bye,
            ),
        })
    }

    /// Takes the tournament's row lock, so anything that reshapes the field or
    /// creates games runs one at a time. Without it a withdrawal and a
    /// round-advance could interleave and pair somebody who has just left.
    pub(crate) async fn lock_row(&self, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table
            .find(self.id)
            .for_update()
            .get_result(conn)
            .await?)
    }

    /// One place to read it, because two callers falling back differently on an
    /// unparseable value would score the table and its tiebreaks in different
    /// units.
    pub(crate) fn scoring_mode(&self) -> ScoringMode {
        ScoringMode::from_str(&self.scoring).unwrap_or(ScoringMode::Game)
    }

    /// `ScoringMode` is the tournament's answer to "do we count matches or
    /// games", so it decides the tiebreak unit too — which only makes a
    /// difference in Double-Swiss, where a 2-0 and a 1½-½ are the same match
    /// result.
    pub(crate) fn tiebreak_basis(&self) -> TiebreakBasis {
        match self.scoring_mode() {
            ScoringMode::Game => TiebreakBasis::GamePoints,
            ScoringMode::Match => TiebreakBasis::MatchPoints,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SeededPlayer {
    pub user_id: Uuid,
    pub seed: i32,
    pub rating: Option<f64>,
    pub withdrawn: bool,
}

/// The tournament's field, and the mapping between hive's user ids and the
/// engine's positional player ids.
pub(crate) struct Field {
    roster: Roster<Uuid>,
    pub(crate) players: Vec<SeededPlayer>,
}

impl Field {
    /// For an arena, whose seeds are arrival order rather than rating order,
    /// so the rank-order identity below deliberately does not hold.
    pub(crate) fn build_in_join_order(players: Vec<SeededPlayer>) -> Result<Self, DbError> {
        let mut roster = Roster::new();
        for player in &players {
            let rating = player
                .rating
                .map(|rating| EngineRating::new(rating.max(0.0).round() as usize));
            roster
                .register(player.user_id, rating)
                .ok_or_else(|| DbError::InvalidAction {
                    info: format!("{} is in the tournament twice", player.user_id),
                })?;
        }
        Ok(Self { roster, players })
    }

    pub(crate) fn build(players: Vec<SeededPlayer>) -> Result<Self, DbError> {
        let mut roster = Roster::new();
        for player in &players {
            let rating = player
                .rating
                .map(|rating| EngineRating::new(rating.max(0.0).round() as usize));
            roster
                .register(player.user_id, rating)
                .ok_or_else(|| DbError::InvalidAction {
                    info: format!("{} is in the tournament twice", player.user_id),
                })?;
        }

        // Seeds are assigned by the same rule the roster ranks by, so the two
        // must agree. If they ever stop agreeing, every stored pairing would be
        // read back against the wrong players, so this is worth refusing over.
        let rank_order = roster.initial_rank_order();
        let identity: Vec<PlayerId> = (0..players.len())
            .map(|index| rank_order[index])
            .enumerate()
            .filter(|(index, player)| player.index() == *index)
            .map(|(_, player)| player)
            .collect();
        if identity.len() != players.len() {
            return Err(DbError::InvalidAction {
                info: String::from("tournament seeding does not match the engine's rank order"),
            });
        }

        Ok(Self { roster, players })
    }

    pub(crate) fn rank_order(&self) -> Vec<PlayerId> {
        self.roster.initial_rank_order()
    }

    pub(crate) fn player_id(&self, user: Uuid) -> Result<PlayerId, DbError> {
        self.roster
            .player_id(user)
            .ok_or_else(|| DbError::InvalidAction {
                info: format!("{user} is not a player in this tournament"),
            })
    }

    pub(crate) fn user_id(&self, player: PlayerId) -> Result<Uuid, DbError> {
        self.roster
            .external_id(player)
            .ok_or_else(|| DbError::InvalidAction {
                info: format!("player {} is not in the roster", player.index()),
            })
    }
}

pub(crate) enum Engine {
    Scored(Box<SwissTournament>),
    Single(Box<SingleEliminationBracket>),
    Double(Box<DoubleEliminationBracket>),
}

impl Engine {
    fn is_round_in_progress(&self) -> bool {
        match self {
            Self::Scored(engine) => engine.is_round_in_progress(),
            Self::Single(bracket) => bracket.is_round_in_progress(),
            Self::Double(bracket) => bracket.is_round_in_progress(),
        }
    }
}

pub(crate) struct Replay {
    pub(crate) field: Field,
    pub(crate) engine: Engine,
    pub(crate) format: EngineFormat,
    /// Pairs whose latest attempt ended without a decisive result and which
    /// have no fresh games waiting. These are what `Replays` re-creates.
    pub(crate) needs_replay: Vec<(Uuid, Uuid)>,
    pub(crate) highest_round: i32,
    /// Every game of the tournament, as replay itself read them. Handed back so
    /// that a caller which needs them too — `standings` does — is not made to
    /// load the whole table a second time.
    pub(crate) games: Vec<Game>,
}

#[derive(Debug)]
pub enum ProgressOutcome {
    /// The round in play still has unfinished games.
    Waiting,
    /// A drawn or double-forfeited match has to be replayed inside its round.
    Replays(Vec<Game>),
    Advanced(Vec<Game>),
    ReadyToFinish,
}

fn stored_bye_kind(kind: ByeKind) -> StoredByeKind {
    match kind {
        ByeKind::PairingAllocated => StoredByeKind::PairingAllocated,
        ByeKind::ZeroPoint => StoredByeKind::ZeroPoint,
    }
}

fn engine_bye_kind(kind: StoredByeKind) -> ByeKind {
    match kind {
        StoredByeKind::PairingAllocated => ByeKind::PairingAllocated,
        StoredByeKind::ZeroPoint => ByeKind::ZeroPoint,
    }
}

fn unordered(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Both games of a two-game match, colours swapped.
fn match_colors(white: Uuid, black: Uuid) -> [(Uuid, Uuid); 2] {
    [(white, black), (black, white)]
}

fn stored_outcome(game: &Game) -> Result<GameOutcome, DbError> {
    let result = TournamentGameResult::from_str(&game.tournament_game_result).map_err(|_| {
        DbError::InvalidAction {
            info: format!(
                "{} is not a valid tournament result",
                game.tournament_game_result
            ),
        }
    })?;
    // A withdrawal forfeit is a game nobody played, exactly like an ordinary
    // forfeit. Calling it played would pay the `loss` rate instead of
    // `forfeit_loss`, and would fold the absentee's phantom results into every
    // opponent's Buchholz, which excludes unplayed games by definition.
    let forfeit = matches!(
        Conclusion::from_str(&game.conclusion),
        Ok(Conclusion::Forfeit) | Ok(Conclusion::Withdrawal)
    );
    let white_score = match result {
        TournamentGameResult::Winner(HiveColor::White) => MatchScore::Win,
        TournamentGameResult::Winner(HiveColor::Black) => MatchScore::Loss,
        TournamentGameResult::Draw => MatchScore::Draw,
        // Not a draw anybody agreed to. The engine reads an *unplayed* draw as
        // the double forfeit it is and pays `forfeit_loss` to both, which is
        // why this pairs with `game_was_played: false` below.
        TournamentGameResult::DoubeForfeit => MatchScore::Draw,
        TournamentGameResult::Unknown => {
            return Err(DbError::InvalidAction {
                info: String::from("cannot score a game with an unknown result"),
            })
        }
    };
    Ok(GameOutcome {
        white_score,
        game_was_played: !forfeit && result != TournamentGameResult::DoubeForfeit,
    })
}

impl Tournament {
    pub fn mode(&self) -> Result<TournamentMode, DbError> {
        TournamentMode::from_str(&self.mode).map_err(|_| DbError::InvalidAction {
            info: format!("{} is not a valid tournament mode", self.mode),
        })
    }

    pub async fn seeded_players(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<SeededPlayer>, DbError> {
        type SeedRow = (Uuid, Option<i32>, Option<f64>, bool, Option<DateTime<Utc>>);
        let rows: Vec<SeedRow> = tournaments_users::table
            .inner_join(users::table)
            .filter(tournaments_users::tournament_id.eq(self.id))
            .select((
                user_id,
                seed_column,
                rating_column,
                users::deleted,
                tournaments_users::withdrawn_at,
            ))
            .order((seed_column.asc().nulls_last(), user_id.asc()))
            .get_results(conn)
            .await?;

        let mut players = Vec::with_capacity(rows.len());
        for (index, (user, seed, rating, deleted, withdrawn_at)) in rows.into_iter().enumerate() {
            // Rows predating the seed column have none. Falling back to the
            // row's position keeps them replayable: their `rating` is null
            // too, so the roster registers everyone unrated and ranks them in
            // registration order, which is the order this just imposed — so
            // `Field::build`'s identity check still holds.
            let seed = seed.unwrap_or(index as i32);
            // `round_robin_schedule` builds player ids straight from slot
            // indices, so a gap here would silently pair the wrong people.
            if seed != index as i32 {
                return Err(DbError::InvalidAction {
                    info: format!(
                        "tournament seeds are not contiguous: expected {index}, got {seed}"
                    ),
                });
            }
            players.push(SeededPlayer {
                user_id: user,
                seed,
                rating,
                // Leaving one tournament and deleting the whole account both
                // mean "do not pair this player again".
                withdrawn: deleted || withdrawn_at.is_some(),
            });
        }
        Ok(players)
    }

    /// Fixes the field's pairing numbers, once, at start. Ordered by rating
    /// descending with unrated last — the same rule `Roster::initial_rank_order`
    /// applies — so the engine's player ids and these seeds always coincide.
    pub(crate) async fn assign_seeds(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<SeededPlayer>, DbError> {
        let speed = GameSpeed::from_base_increment(self.time_base, self.time_increment);
        let players = self.players(conn).await?;

        let ids: Vec<Uuid> = players.iter().map(|player| player.id).collect();
        let ratings: HashMap<Uuid, f64> = Rating::for_uuids_at_speed(&ids, &speed, conn)
            .await?
            .into_iter()
            .map(|rating| (rating.user_uid, rating.rating))
            .collect();
        let mut rated: Vec<(Uuid, Option<f64>)> = players
            .iter()
            .map(|player| (player.id, ratings.get(&player.id).copied()))
            .collect();
        // An arena indexes players by arrival, and admits more later, so its
        // seeds must follow the join order rather than strength.
        if self.mode()?.is_arena() {
            let joined: HashMap<Uuid, Option<DateTime<Utc>>> = tournaments_users::table
                .filter(tournaments_users::tournament_id.eq(self.id))
                .select((user_id, tournaments_users::joined_at))
                .get_results::<(Uuid, Option<DateTime<Utc>>)>(conn)
                .await?
                .into_iter()
                .collect();
            rated.sort_by(|(left_id, _), (right_id, _)| {
                joined
                    .get(left_id)
                    .copied()
                    .flatten()
                    .cmp(&joined.get(right_id).copied().flatten())
                    .then_with(|| left_id.cmp(right_id))
            });
        } else {
            rated.sort_by(|(left_id, left), (right_id, right)| {
                right
                    .partial_cmp(left)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left_id.cmp(right_id))
            });
        }

        let mut seeded = Vec::with_capacity(rated.len());
        for (seed, (user, rating)) in rated.into_iter().enumerate() {
            diesel::update(
                tournaments_users::table.filter(
                    tournaments_users::tournament_id
                        .eq(self.id)
                        .and(user_id.eq(user)),
                ),
            )
            .set((seed_column.eq(Some(seed as i32)), rating_column.eq(rating)))
            .execute(conn)
            .await?;
            seeded.push(SeededPlayer {
                user_id: user,
                seed: seed as i32,
                rating,
                withdrawn: false,
            });
        }
        Ok(seeded)
    }

    async fn tournament_games_in_order(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        Ok(games::table
            .filter(tournament_id.eq(Some(self.id)))
            .order((round_column.asc(), created_at.asc(), game_id_column.asc()))
            .get_results(conn)
            .await?)
    }

    fn expected_rounds(&self, format: EngineFormat, player_count: usize) -> usize {
        match format {
            EngineFormat::RoundRobin { repeats } => {
                let per_pass = if player_count.is_multiple_of(2) {
                    player_count.saturating_sub(1)
                } else {
                    player_count
                };
                per_pass * repeats
            }
            _ => self.rounds.max(1) as usize,
        }
    }

    pub(crate) async fn replay(&self, conn: &mut DbConn<'_>) -> Result<Replay, DbError> {
        let format = EngineFormat::for_tournament(self)?;
        let field = Field::build(self.seeded_players(conn).await?)?;
        let all_games = self.tournament_games_in_order(conn).await?;
        let stored_byes = TournamentBye::for_tournament(self.id, conn).await?;

        let mut rounds: BTreeMap<i32, Vec<&Game>> = BTreeMap::new();
        for game in &all_games {
            // Skipping these would be worse than refusing: a round robin would
            // then score nobody at all and report an all-zero tie rather than
            // an error. Every format needs the backfill.
            let Some(round) = game.round else {
                return Err(DbError::InvalidAction {
                    info: String::from(
                        "tournament game is missing a round number; run the round backfill",
                    ),
                });
            };
            rounds.entry(round).or_default().push(game);
        }

        let mut byes_by_round: BTreeMap<i32, Vec<(Uuid, StoredByeKind)>> = BTreeMap::new();
        for bye in stored_byes {
            let kind = bye.kind()?;
            byes_by_round
                .entry(bye.round)
                .or_default()
                .push((bye.user_id, kind));
        }

        // A round can consist of nothing but a bye — a lone pairable survivor
        // gets one and no game is created. Keying only off the games would
        // leave that round invisible, so its bye would never score and the
        // round counter would never move past it. Brackets store no byes; an
        // empty round would read to them as one that finished.
        //
        // Only a pairing-allocated bye can stand for a round on its own,
        // because only pairing produces one. A zero-point bye is granted
        // *before* its round is paired, so treating it as a round would begin
        // that round early, with nothing in it but the player sitting out.
        if !format.is_bracket() {
            let paired_rounds: Vec<i32> = byes_by_round
                .iter()
                .filter(|(_, byes)| {
                    byes.iter()
                        .any(|(_, kind)| *kind == StoredByeKind::PairingAllocated)
                })
                .map(|(round, _)| *round)
                .collect();
            for round in paired_rounds {
                rounds.entry(round).or_default();
            }
        }

        let highest_round = rounds.keys().copied().max().unwrap_or(0);
        let mut needs_replay: Vec<(Uuid, Uuid)> = Vec::new();

        let engine = match format {
            EngineFormat::Swiss(system) => {
                let mut engine = SwissTournament::new(
                    &field.rank_order(),
                    system,
                    self.point_system(format),
                    Color::White,
                    RoundIndex::new(self.expected_rounds(format, field.players.len())),
                )
                .map_err(|error| DbError::InvalidAction {
                    info: format!("could not build the Swiss tournament: {error}"),
                })?;
                engine.set_tiebreak_basis(self.tiebreak_basis());
                // Before the results, not after: a Double-Swiss match whose two
                // games were both forfeited by a withdrawal has to be read as a
                // walkover rather than a replay, and the engine can only tell
                // the difference if it already knows who has left.
                withdraw_from(&mut engine, &field)?;
                self.replay_scored(
                    &mut engine,
                    &field,
                    format,
                    &rounds,
                    &byes_by_round,
                    &mut needs_replay,
                )?;
                Engine::Scored(Box::new(engine))
            }
            EngineFormat::RoundRobin { .. } => {
                let mut engine = SwissTournament::new(
                    &field.rank_order(),
                    SwissSystem::None,
                    self.point_system(format),
                    Color::White,
                    RoundIndex::new(self.expected_rounds(format, field.players.len())),
                )
                .map_err(|error| DbError::InvalidAction {
                    info: format!("could not build the round robin: {error}"),
                })?;
                engine.set_tiebreak_basis(self.tiebreak_basis());
                // A round robin's games all exist already, so withdrawal shows
                // up as forfeited results rather than absent pairings — but the
                // engine is still told, so anything that asks it whether a
                // player is active gets the same answer as the database.
                withdraw_from(&mut engine, &field)?;
                self.replay_scored(
                    &mut engine,
                    &field,
                    format,
                    &rounds,
                    &byes_by_round,
                    &mut needs_replay,
                )?;
                Engine::Scored(Box::new(engine))
            }
            EngineFormat::SingleElimination { third_place } => {
                let mut bracket = SingleEliminationBracket::new(&field.rank_order(), third_place)
                    .map_err(|error| DbError::InvalidAction {
                    info: format!("could not build the bracket: {error}"),
                })?;
                for round in rounds.values() {
                    if bracket.next_round().is_err() {
                        break;
                    }
                    if !replay_bracket_round(
                        &field,
                        round,
                        &mut needs_replay,
                        |white, black, outcome| bracket.record_result(white, black, outcome),
                    )? {
                        break;
                    }
                }
                withdraw_from_bracket(|player| bracket.withdraw_player(player), &field)?;
                Engine::Single(Box::new(bracket))
            }
            EngineFormat::DoubleElimination => {
                let mut bracket =
                    DoubleEliminationBracket::new(&field.rank_order()).map_err(|error| {
                        DbError::InvalidAction {
                            info: format!("could not build the bracket: {error}"),
                        }
                    })?;
                for round in rounds.values() {
                    if bracket.next_round().is_err() {
                        break;
                    }
                    if !replay_bracket_round(
                        &field,
                        round,
                        &mut needs_replay,
                        |white, black, outcome| bracket.record_result(white, black, outcome),
                    )? {
                        break;
                    }
                }
                withdraw_from_bracket(|player| bracket.withdraw_player(player), &field)?;
                Engine::Double(Box::new(bracket))
            }
        };

        // A pairing involving somebody who has left is never replayed: there is
        // nobody to play it. Both engines resolve it as a walkover instead — a
        // bracket through `award`, Swiss through `double_swiss_match_outcome`'s
        // withdrawal case — but a bracket only learns who withdrew after its
        // rounds have been fed in, so by then this list can already name them.
        let departed: HashSet<Uuid> = field
            .players
            .iter()
            .filter(|player| player.withdrawn)
            .map(|player| player.user_id)
            .collect();
        needs_replay.retain(|(left, right)| !departed.contains(left) && !departed.contains(right));

        Ok(Replay {
            field,
            engine,
            format,
            needs_replay,
            highest_round,
            games: all_games,
        })
    }

    fn replay_scored(
        &self,
        engine: &mut SwissTournament,
        field: &Field,
        format: EngineFormat,
        rounds: &BTreeMap<i32, Vec<&Game>>,
        byes_by_round: &BTreeMap<i32, Vec<(Uuid, StoredByeKind)>>,
        needs_replay: &mut Vec<(Uuid, Uuid)>,
    ) -> Result<(), DbError> {
        for (round, round_games) in rounds {
            let pairs = distinct_pairs(round_games);

            // Sequential formats declare the round as it was actually paired,
            // so an unfinished game leaves the engine's round open — exactly
            // mirroring the database. Round robin declares only what has
            // finished, so later rounds can still be scored.
            let (declared, playable): (Vec<(Uuid, Uuid)>, Vec<&Game>) = if format.is_sequential() {
                (
                    pairs.clone(),
                    round_games
                        .iter()
                        .copied()
                        .filter(|game| game.finished)
                        .collect(),
                )
            } else {
                let complete: Vec<(Uuid, Uuid)> = pairs
                    .iter()
                    .copied()
                    .filter(|pair| {
                        round_games
                            .iter()
                            .filter(|game| unordered(game.white_id, game.black_id) == *pair)
                            .all(|game| game.finished)
                    })
                    .collect();
                let games = round_games
                    .iter()
                    .copied()
                    .filter(|game| {
                        game.finished && complete.contains(&unordered(game.white_id, game.black_id))
                    })
                    .collect();
                (complete, games)
            };

            let byes: Vec<ByeAssignment> = byes_by_round
                .get(round)
                .map(|players| {
                    players
                        .iter()
                        .map(|(user, kind)| {
                            Ok(ByeAssignment {
                                player: field.player_id(*user)?,
                                kind: engine_bye_kind(*kind),
                            })
                        })
                        .collect::<Result<Vec<_>, DbError>>()
                })
                .transpose()?
                .unwrap_or_default();

            // Beginning an empty round would close it immediately and advance
            // the played-round counter for nothing.
            if declared.is_empty() && byes.is_empty() {
                continue;
            }

            let pairings: Vec<Pairing> = declared
                .iter()
                .map(|(left, right)| {
                    let orientation = round_games
                        .iter()
                        .find(|game| unordered(game.white_id, game.black_id) == (*left, *right))
                        .ok_or_else(|| DbError::InvalidAction {
                            info: String::from("a declared pairing has no game"),
                        })?;
                    Ok(Pairing::new(
                        field.player_id(orientation.white_id)?,
                        field.player_id(orientation.black_id)?,
                    ))
                })
                .collect::<Result<Vec<_>, DbError>>()?;

            engine
                .begin_round(&byes, pairings)
                .map_err(|error| DbError::InvalidAction {
                    info: format!("could not begin round {round}: {error}"),
                })?;

            for game in playable {
                let pair = unordered(game.white_id, game.black_id);
                let white = field.player_id(game.white_id)?;
                let black = field.player_id(game.black_id)?;
                match engine.record_result(white, black, stored_outcome(game)?) {
                    Ok(_) => needs_replay.retain(|existing| *existing != pair),
                    // Not a failure: the pair drew or double-forfeited and has
                    // to play again. Later rows in this round are that replay.
                    Err(SubmitError::NeedsReplay) => {
                        if !needs_replay.contains(&pair) {
                            needs_replay.push(pair);
                        }
                    }
                    Err(error) => {
                        return Err(DbError::InvalidAction {
                            info: format!("could not record a result in round {round}: {error}"),
                        })
                    }
                }
            }

            // The replay games a previous tick already created are unfinished,
            // so the loop above never saw them and never cleared the pair.
            // Without this every later tick creates another match for it.
            needs_replay.retain(|pair| {
                !round_games
                    .iter()
                    .any(|game| !game.finished && unordered(game.white_id, game.black_id) == *pair)
            });
        }
        Ok(())
    }
}

/// Tells a scored engine about everyone who has left, once the history is in —
/// withdrawal only affects who gets paired next, never what has already been
/// scored.
fn withdraw_from(engine: &mut SwissTournament, field: &Field) -> Result<(), DbError> {
    for player in field.players.iter().filter(|player| player.withdrawn) {
        engine
            .withdraw_player(field.player_id(player.user_id)?)
            .map_err(|error| DbError::InvalidAction {
                info: format!("could not withdraw a player: {error}"),
            })?;
    }
    Ok(())
}

/// The same, for a bracket. A knockout slot has to produce somebody, so the
/// engine resolves their matches as walkovers rather than skipping them; hive
/// creates and forfeits the matching game rows so the two agree.
fn withdraw_from_bracket(
    mut withdraw: impl FnMut(PlayerId) -> Option<Pairing>,
    field: &Field,
) -> Result<(), DbError> {
    for player in field.players.iter().filter(|player| player.withdrawn) {
        withdraw(field.player_id(player.user_id)?);
    }
    Ok(())
}

/// Distinct unordered pairs of a round, in the order their games first appear.
/// A two-game match is one pair, not two.
fn distinct_pairs(round_games: &[&Game]) -> Vec<(Uuid, Uuid)> {
    let mut seen = HashSet::new();
    let mut pairs = Vec::new();
    for game in round_games {
        let pair = unordered(game.white_id, game.black_id);
        if seen.insert(pair) {
            pairs.push(pair);
        }
    }
    pairs
}

/// Returns whether the round closed, so the caller knows to stop feeding the
/// bracket once it hits a round still being played.
fn replay_bracket_round(
    field: &Field,
    round_games: &[&Game],
    needs_replay: &mut Vec<(Uuid, Uuid)>,
    mut record: impl FnMut(
        PlayerId,
        PlayerId,
        GameOutcome,
    ) -> Result<tournamint::RoundStatus, EliminationSubmitError>,
) -> Result<bool, DbError> {
    let mut all_finished = true;
    for game in round_games {
        if !game.finished {
            all_finished = false;
            continue;
        }
        let pair = unordered(game.white_id, game.black_id);
        let white = field.player_id(game.white_id)?;
        let black = field.player_id(game.black_id)?;
        match record(white, black, stored_outcome(game)?) {
            Ok(_) => needs_replay.retain(|existing| *existing != pair),
            Err(EliminationSubmitError::NeedsAnotherAttempt) => {
                if !needs_replay.contains(&pair) {
                    needs_replay.push(pair);
                }
            }
            Err(error) => {
                return Err(DbError::InvalidAction {
                    info: format!("could not record a bracket result: {error}"),
                })
            }
        }
    }

    // The replay games a previous tick already created are unfinished, so the
    // loop above skipped them and never cleared the pair. Without this every
    // later tick creates another attempt for it.
    needs_replay.retain(|pair| {
        !round_games
            .iter()
            .any(|game| !game.finished && unordered(game.white_id, game.black_id) == *pair)
    });

    Ok(all_finished)
}

impl Tournament {
    /// Creates the games the tournament opens with. Round robin's whole
    /// schedule is known up front and is created in one go; the other formats
    /// only know their first round.
    pub(crate) async fn create_initial_games(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        // An arena opens with nobody paired: the first pairing happens on the
        // first tick, once `started_at` exists to measure its clock against.
        if self.mode()?.is_arena() {
            self.assign_seeds(conn).await?;
            return Ok(Vec::new());
        }
        let format = EngineFormat::for_tournament(self)?;
        let field = Field::build(self.assign_seeds(conn).await?)?;
        let player_count = field.players.len();

        match format {
            EngineFormat::RoundRobin { repeats } => {
                let schedule = round_robin_schedule(player_count, repeats).map_err(|error| {
                    DbError::InvalidAction {
                        info: format!("could not build a round robin schedule: {error}"),
                    }
                })?;
                let total_rounds = schedule.len();
                let mut games = Vec::new();
                for (index, round) in schedule.into_iter().enumerate() {
                    let number = index as i32 + 1;
                    games.extend(
                        self.create_round_games(&field, &round.games, number, false, conn)
                            .await?,
                    );
                    self.record_byes(&field, &round.byes, number, conn).await?;
                }
                self.set_rounds(total_rounds as i32, conn).await?;
                Ok(games)
            }
            EngineFormat::Swiss(_) => {
                let Replay { mut engine, .. } = self.replay(conn).await?;
                let Engine::Scored(engine) = &mut engine else {
                    return Err(DbError::InvalidAction {
                        info: String::from("expected a scored engine"),
                    });
                };
                let round = pair_next_round(engine).map_err(|error| DbError::InvalidAction {
                    info: format!("could not pair the first round: {error}"),
                })?;
                let games = self
                    .create_round_games(
                        &field,
                        &round.games,
                        1,
                        format.plays_two_game_matches(),
                        conn,
                    )
                    .await?;
                let byes: Vec<PlayerId> = round.byes.iter().map(|bye| bye.player).collect();
                self.record_bye_players(&field, &byes, 1, conn).await?;
                Ok(games)
            }
            EngineFormat::SingleElimination { third_place } => {
                let mut bracket = SingleEliminationBracket::new(&field.rank_order(), third_place)
                    .map_err(|error| DbError::InvalidAction {
                    info: format!("could not build the bracket: {error}"),
                })?;
                let round = bracket
                    .next_round()
                    .map_err(|error| DbError::InvalidAction {
                        info: format!("could not pair the first bracket round: {error}"),
                    })?;
                self.set_rounds(bracket_round_count(player_count), conn)
                    .await?;
                self.create_round_games(&field, &round.matches, 1, true, conn)
                    .await
            }
            EngineFormat::DoubleElimination => {
                let mut bracket =
                    DoubleEliminationBracket::new(&field.rank_order()).map_err(|error| {
                        DbError::InvalidAction {
                            info: format!("could not build the bracket: {error}"),
                        }
                    })?;
                let round = bracket
                    .next_round()
                    .map_err(|error| DbError::InvalidAction {
                        info: format!("could not pair the first bracket round: {error}"),
                    })?;
                self.set_rounds(1, conn).await?;
                self.create_round_games(&field, &round.matches, 1, true, conn)
                    .await
            }
        }
    }

    async fn set_rounds(&self, rounds: i32, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::update(tournaments::table.find(self.id))
            .set((rounds_column.eq(rounds), updated_at.eq(Utc::now())))
            .execute(conn)
            .await?;
        Ok(())
    }

    /// `rounds` is the tournament's *total*, which is what "round 2 of 5" reads
    /// from. A single-elimination bracket knows its total up front; a double
    /// elimination one does not, so its total is the furthest round reached so
    /// far. Either way advancing must never lower it.
    async fn raise_rounds_to(&self, round: i32, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::update(
            tournaments::table.filter(tournaments::id.eq(self.id).and(rounds_column.lt(round))),
        )
        .set((rounds_column.eq(round), updated_at.eq(Utc::now())))
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn create_round_games(
        &self,
        field: &Field,
        pairings: &[Pairing],
        round: i32,
        two_game_match: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        let gone: HashSet<Uuid> = field
            .players
            .iter()
            .filter(|player| player.withdrawn)
            .map(|player| player.user_id)
            .collect();

        let mut games = Vec::new();
        for pairing in pairings {
            let white = field.user_id(pairing.white())?;
            let black = field.user_id(pairing.black())?;
            let colors: &[(Uuid, Uuid)] = if two_game_match {
                &match_colors(white, black)
            } else {
                &[(white, black)]
            };
            for (white, black) in colors {
                let game = Game::create(
                    NewGame::new_from_tournament(*white, *black, self, round),
                    conn,
                )
                .await?;
                // Swiss keeps a deleted account out of the draw entirely, but a
                // bracket has no withdrawal: the slot exists and somebody has to
                // advance out of it. So the game is created and forfeited at
                // once, rather than sitting unplayable forever.
                let game = match (gone.contains(white), gone.contains(black)) {
                    (false, false) => game,
                    (true, false) => {
                        game.assign_tournament_result(
                            &TournamentGameResult::Winner(HiveColor::Black),
                            conn,
                        )
                        .await?
                    }
                    (false, true) => {
                        game.assign_tournament_result(
                            &TournamentGameResult::Winner(HiveColor::White),
                            conn,
                        )
                        .await?
                    }
                    (true, true) => {
                        game.assign_tournament_result(&TournamentGameResult::DoubeForfeit, conn)
                            .await?
                    }
                };
                games.push(game);
            }
        }
        Ok(games)
    }

    async fn record_byes(
        &self,
        field: &Field,
        byes: &[ByeAssignment],
        round: i32,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let rows = byes
            .iter()
            .map(|bye| {
                Ok(TournamentBye::new(
                    self.id,
                    round,
                    field.user_id(bye.player)?,
                    stored_bye_kind(bye.kind),
                ))
            })
            .collect::<Result<Vec<_>, DbError>>()?;
        TournamentBye::insert_many(&rows, conn).await
    }

    /// Byes the engine handed out itself are always the odd-player-out kind;
    /// a zero-point bye is granted by an organizer, never paired.
    async fn record_bye_players(
        &self,
        field: &Field,
        byes: &[PlayerId],
        round: i32,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let rows = byes
            .iter()
            .map(|player| {
                Ok(TournamentBye::new(
                    self.id,
                    round,
                    field.user_id(*player)?,
                    StoredByeKind::PairingAllocated,
                ))
            })
            .collect::<Result<Vec<_>, DbError>>()?;
        TournamentBye::insert_many(&rows, conn).await
    }

    /// Sits a player out of a round they have not been paired into yet, for
    /// `points_zero_point_bye` — the FIDE requested bye. Organizers only.
    ///
    /// Only the round about to be paired can be granted one: the engine scores
    /// a bye at `begin_round`, so a round already in the history would have to
    /// be re-scored, and a round further ahead does not exist yet.
    pub async fn grant_zero_point_bye(
        &self,
        user: &Uuid,
        organizer: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentBye, DbError> {
        let locked = self.lock_row(conn).await?;
        locked.ensure_inprogress()?;
        locked
            .ensure_user_is_organizer_or_admin(organizer, conn)
            .await?;

        if locked.mode()?.is_arena() {
            return Err(DbError::InvalidAction {
                info: String::from("an arena has no rounds to sit out"),
            });
        }

        let replay = locked.replay(conn).await?;
        if !replay
            .field
            .players
            .iter()
            .any(|player| player.user_id == *user && !player.withdrawn)
        {
            return Err(DbError::InvalidAction {
                info: String::from("that player is not active in this tournament"),
            });
        }
        if replay.engine.is_round_in_progress() {
            return Err(DbError::InvalidAction {
                info: String::from(
                    "the round is already paired; grant the bye before it is created",
                ),
            });
        }

        let round = replay.highest_round + 1;
        let bye = TournamentBye::new(locked.id, round, *user, StoredByeKind::ZeroPoint);
        TournamentBye::insert_many(std::slice::from_ref(&bye), conn).await?;
        Ok(bye)
    }

    /// Drops a player out of one tournament, without touching their account.
    /// Either the player themselves or an organizer may do it.
    ///
    /// What they have already scored stands and keeps counting toward everyone
    /// else's tiebreaks. What happens to their *unplayed* games depends on the
    /// format: Swiss simply stops pairing them, a bracket walks their opponent
    /// over, and a round robin — whose whole schedule already exists — forfeits
    /// the rest. An arena records it on its own timeline, since pairing there
    /// depends on exactly when it happened.
    pub async fn withdraw_player(
        &self,
        user: &Uuid,
        actor: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        // Every guard below has to read the row the lock just returned. Checking
        // the caller's stale copy would let a withdrawal that waited on the lock
        // pass an `InProgress` check the finisher ahead of it had already
        // invalidated, and write into a finished tournament.
        let locked = self.lock_row(conn).await?;
        locked.ensure_inprogress()?;
        if actor != user {
            locked
                .ensure_user_is_organizer_or_admin(actor, conn)
                .await?;
        }

        // The arena event has to be written *before* `withdrawn_at`, not after.
        // Replay treats a set `withdrawn_at` as a withdrawal in its own right,
        // so writing the column first would make the replay inside
        // `withdraw_from_arena` withdraw them, and the real call would then be
        // refused as a repeat.
        let arena = locked.mode()?.is_arena();
        if arena {
            locked.withdraw_from_arena(user, conn).await?;
        }

        let updated = diesel::update(
            tournaments_users::table.filter(
                tournaments_users::tournament_id
                    .eq(locked.id)
                    .and(user_id.eq(user))
                    .and(tournaments_users::withdrawn_at.is_null()),
            ),
        )
        .set(tournaments_users::withdrawn_at.eq(Some(Utc::now())))
        .execute(conn)
        .await?;
        if updated == 0 {
            return Err(DbError::InvalidAction {
                info: String::from("not in this tournament, or already withdrawn"),
            });
        }

        // An arena game already under way keeps its own clock and is left to
        // finish or time out — that is what the engine expects and what lichess
        // does. But a game that was paired and never started has no clock at
        // all (`compute_timeout_at` gives an unstarted game no `timeout_at`, so
        // the sweeper never sees it), and would otherwise sit in flight
        // forever, keeping the arena from ever finishing.
        locked.forfeit_remaining_games(user, !arena, conn).await
    }

    /// Puts a withdrawn player back in, and un-forfeits exactly the games the
    /// withdrawal forfeited — which is what `Conclusion::Withdrawal` is for,
    /// so an organizer's own adjudications are left alone. Organizers only.
    pub async fn reinstate_player(
        &self,
        user: &Uuid,
        organizer: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        // As in `withdraw_player`: the guards must read the locked row, not the
        // caller's snapshot of it.
        let locked = self.lock_row(conn).await?;
        locked.ensure_inprogress()?;
        locked
            .ensure_user_is_organizer_or_admin(organizer, conn)
            .await?;

        let updated = diesel::update(
            tournaments_users::table.filter(
                tournaments_users::tournament_id
                    .eq(locked.id)
                    .and(user_id.eq(user))
                    .and(tournaments_users::withdrawn_at.is_not_null()),
            ),
        )
        .set(tournaments_users::withdrawn_at.eq(None::<DateTime<Utc>>))
        .execute(conn)
        .await?;
        if updated == 0 {
            return Err(DbError::InvalidAction {
                info: String::from("that player has not withdrawn"),
            });
        }

        // The arena engine has no un-withdraw, so the event is dropped rather
        // than reversed: replay simply never sees it. The tournaments_users row
        // remains the record that they were once out.
        if locked.mode()?.is_arena() {
            TournamentArenaEvent::forget(locked.id, *user, ArenaEventKind::Withdraw, conn).await?;
        }

        locked.restore_withdrawn_games(user, conn).await
    }

    /// Forfeits every unfinished game a departing player still has, marked so
    /// that reinstating them can undo precisely these.
    async fn forfeit_remaining_games(
        &self,
        user: &Uuid,
        resign_games_in_progress: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        let games: Vec<Game> = games::table
            .filter(tournament_id.eq(Some(self.id)))
            .filter(games::finished.eq(false))
            .filter(games::white_id.eq(user).or(games::black_id.eq(user)))
            .get_results(conn)
            .await?;

        let mut forfeited = 0;
        for game in games {
            let color = game
                .user_color(*user)
                .ok_or_else(|| DbError::InvalidAction {
                    info: String::from("a game of theirs does not have them as a player"),
                })?;

            // A game already under way is a real game. Outside an arena it gets
            // resigned, with the rating consequences that carries, exactly as
            // deleting an account does; inside one it is left alone to run its
            // clock down. Either way only a game nobody has touched is
            // forfeited as a withdrawal, which is the reversible kind.
            if game.turn > 0 || !game.history.is_empty() {
                if resign_games_in_progress {
                    game.resign(&hive_lib::GameControl::Resign(color), conn)
                        .await?;
                }
                continue;
            }

            diesel::update(games::table.find(game.id))
                .set((
                    games::finished.eq(true),
                    games::game_status.eq(hive_lib::GameStatus::Adjudicated.to_string()),
                    games::conclusion.eq(Conclusion::Withdrawal.to_string()),
                    games::tournament_game_result
                        .eq(TournamentGameResult::Winner(color.opposite_color()).to_string()),
                    games::updated_at.eq(Utc::now()),
                    games::finished_at.eq(Utc::now()),
                ))
                .execute(conn)
                .await?;
            forfeited += 1;
        }
        Ok(forfeited)
    }

    async fn restore_withdrawn_games(
        &self,
        user: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        let withdrawn: Vec<Game> = games::table
            .filter(tournament_id.eq(Some(self.id)))
            .filter(games::conclusion.eq(Conclusion::Withdrawal.to_string()))
            .filter(games::white_id.eq(user).or(games::black_id.eq(user)))
            .get_results(conn)
            .await?;

        if withdrawn.is_empty() {
            return Ok(0);
        }

        // Anyone else who is still out. Un-forfeiting a game against them would
        // leave a game neither side is ever going to play, which no format can
        // then finish.
        let still_out: HashSet<Uuid> = self
            .seeded_players(conn)
            .await?
            .into_iter()
            .filter(|player| player.withdrawn)
            .map(|player| player.user_id)
            .collect();

        // A round the tournament has already moved past is settled. Re-opening
        // a game in one would leave that round permanently unresolved in
        // replay, and the next `begin_round` would refuse because the round it
        // is holding never closed. Round robin pairs every round up front and
        // scores each pair independently, and an arena has no rounds at all, so
        // neither has an ordering to break.
        let sequential =
            !self.mode()?.is_arena() && EngineFormat::for_tournament(self)?.is_sequential();
        let current_round: Option<i32> = if sequential {
            games::table
                .filter(tournament_id.eq(Some(self.id)))
                .select(diesel::dsl::max(round_column))
                .first(conn)
                .await?
        } else {
            None
        };

        let restorable: Vec<Uuid> = withdrawn
            .iter()
            .filter(|game| {
                let opponent = if game.white_id == *user {
                    game.black_id
                } else {
                    game.white_id
                };
                !still_out.contains(&opponent)
                    && current_round.is_none_or(|current| game.round == Some(current))
            })
            .map(|game| game.id)
            .collect();

        if restorable.is_empty() {
            return Ok(0);
        }

        let restored = diesel::update(games::table.filter(games::id.eq_any(&restorable)))
            .set((
                games::finished.eq(false),
                games::game_status.eq(hive_lib::GameStatus::NotStarted.to_string()),
                games::conclusion.eq(Conclusion::Unknown.to_string()),
                games::tournament_game_result.eq(TournamentGameResult::Unknown.to_string()),
                games::updated_at.eq(Utc::now()),
                games::last_interaction.eq::<Option<DateTime<Utc>>>(None),
                games::turn.eq(0),
                games::timeout_at.eq(crate::models::game::CLEAR_TIMEOUT_AT),
                games::finished_at.eq::<Option<DateTime<Utc>>>(None),
            ))
            .execute(conn)
            .await?;

        Schedule::delete_for_games(&restorable, conn).await?;

        Ok(restored)
    }

    pub async fn progress_by_organizer(
        &self,
        organizer: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<ProgressOutcome, DbError> {
        self.ensure_user_is_organizer_or_admin(organizer, conn)
            .await?;
        self.progress(conn).await
    }

    /// Idempotent: safe to call from a job and from an organizer at the same
    /// time, and safe to call repeatedly.
    pub async fn progress(&self, conn: &mut DbConn<'_>) -> Result<ProgressOutcome, DbError> {
        // Without this two callers would both see the round complete and both
        // create the next one.
        let locked: Tournament = tournaments::table
            .find(self.id)
            .for_update()
            .get_result(conn)
            .await?;
        locked.ensure_inprogress()?;

        // An arena has no rounds to advance: a tick pairs whoever is waiting,
        // and it is over when the clock runs out with no game still going.
        if locked.mode()?.is_arena() {
            let now = Utc::now();
            let paired = locked.pair_arena(now, conn).await?;
            if !paired.is_empty() {
                return Ok(ProgressOutcome::Advanced(paired));
            }
            return if locked.arena_is_over(now, conn).await? {
                Ok(ProgressOutcome::ReadyToFinish)
            } else {
                Ok(ProgressOutcome::Waiting)
            };
        }

        let Replay {
            field,
            mut engine,
            format,
            needs_replay,
            highest_round,
            ..
        } = locked.replay(conn).await?;

        if !needs_replay.is_empty() {
            let mut games = Vec::new();
            for (left, right) in &needs_replay {
                for (white, black) in match_colors(*left, *right) {
                    games.push(
                        Game::create(
                            NewGame::new_from_tournament(white, black, &locked, highest_round),
                            conn,
                        )
                        .await?,
                    );
                }
            }
            return Ok(ProgressOutcome::Replays(games));
        }

        if engine.is_round_in_progress() {
            return Ok(ProgressOutcome::Waiting);
        }

        if matches!(format, EngineFormat::RoundRobin { .. }) {
            // The whole schedule already exists, so there is never a next round
            // to pair — only the whole thing being over.
            return if locked.number_of_games(conn).await?
                == locked.number_of_finished_games(conn).await?
            {
                Ok(ProgressOutcome::ReadyToFinish)
            } else {
                Ok(ProgressOutcome::Waiting)
            };
        }

        let number = highest_round + 1;
        match &mut engine {
            Engine::Scored(engine) => {
                if engine.played_rounds().value() >= engine.expected_rounds().value() {
                    return Ok(ProgressOutcome::ReadyToFinish);
                }
                // Anyone granted a zero-point bye for this round must not be
                // paired into it. The engine does not persist that across a
                // replay, so it is re-applied here, right before pairing.
                for bye in TournamentBye::for_round(locked.id, number, conn).await? {
                    if bye.kind()? == StoredByeKind::ZeroPoint {
                        engine
                            .sit_out_next_round(field.player_id(bye.user_id)?)
                            .map_err(|error| DbError::InvalidAction {
                                info: format!("could not sit a player out: {error}"),
                            })?;
                    }
                }
                let round = pair_next_round(engine).map_err(|error| DbError::InvalidAction {
                    info: format!("could not pair the next round: {error}"),
                })?;
                let games = locked
                    .create_round_games(
                        &field,
                        &round.games,
                        number,
                        format.plays_two_game_matches(),
                        conn,
                    )
                    .await?;
                locked
                    .record_byes(&field, &round.byes, number, conn)
                    .await?;
                Ok(ProgressOutcome::Advanced(games))
            }
            Engine::Single(bracket) => {
                if bracket.champion().is_some() {
                    return Ok(ProgressOutcome::ReadyToFinish);
                }
                let matches = match bracket.next_round() {
                    Ok(round) => round.matches,
                    Err(NextRoundError::TournamentAlreadyOver) => {
                        return Ok(ProgressOutcome::ReadyToFinish)
                    }
                    Err(error) => {
                        return Err(DbError::InvalidAction {
                            info: format!("could not pair the next bracket round: {error}"),
                        })
                    }
                };
                locked
                    .create_bracket_round(&field, &matches, number, conn)
                    .await
            }
            Engine::Double(bracket) => {
                if bracket.champion().is_some() {
                    return Ok(ProgressOutcome::ReadyToFinish);
                }
                let matches = match bracket.next_round() {
                    Ok(round) => round.matches,
                    Err(NextRoundError::TournamentAlreadyOver) => {
                        return Ok(ProgressOutcome::ReadyToFinish)
                    }
                    Err(error) => {
                        return Err(DbError::InvalidAction {
                            info: format!("could not pair the next bracket round: {error}"),
                        })
                    }
                };
                locked
                    .create_bracket_round(&field, &matches, number, conn)
                    .await
            }
        }
    }

    async fn create_bracket_round(
        &self,
        field: &Field,
        matches: &[Pairing],
        round: i32,
        conn: &mut DbConn<'_>,
    ) -> Result<ProgressOutcome, DbError> {
        if matches.is_empty() {
            return Ok(ProgressOutcome::ReadyToFinish);
        }
        let games = self
            .create_round_games(field, matches, round, true, conn)
            .await?;
        self.raise_rounds_to(round, conn).await?;
        Ok(ProgressOutcome::Advanced(games))
    }

    async fn in_progress_and_fully_automated(conn: &mut DbConn<'_>) -> Result<Vec<Self>, DbError> {
        Ok(tournaments::table
            .filter(
                tournaments::status
                    .eq(TournamentStatus::InProgress.to_string())
                    .and(tournaments::fully_automated.eq(true)),
            )
            .get_results(conn)
            .await?)
    }

    pub async fn automatic_progress(
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(Tournament, ProgressOutcome)>, DbError> {
        let mut progressed = Vec::new();
        for tournament in Self::in_progress_and_fully_automated(conn).await? {
            // Each tournament gets its own transaction, for two reasons. The
            // row lock `progress` takes only holds for the length of one
            // statement in autocommit, so without this the serialization it
            // relies on is not there at all. And one tournament that has wedged
            // itself must not stop every other automated tournament on the
            // site — it is logged and skipped instead.
            let id = tournament.id;
            let outcome = conn
                .transaction::<_, DbError, _>(async move |tc| {
                    match tournament.progress(tc).await? {
                        ProgressOutcome::Waiting => Ok(None),
                        ProgressOutcome::ReadyToFinish => {
                            let finished = tournament.finish_automatically(tc).await?;
                            Ok(Some((finished, ProgressOutcome::ReadyToFinish)))
                        }
                        outcome => Ok(Some((tournament, outcome))),
                    }
                })
                .await;

            match outcome {
                Ok(Some(entry)) => progressed.push(entry),
                Ok(None) => {}
                Err(error) => {
                    tracing::error!(
                        tournament_id = %id,
                        %error,
                        "automatic progress failed for this tournament; skipping it",
                    );
                }
            }
        }
        Ok(progressed)
    }
}

fn bracket_round_count(player_count: usize) -> i32 {
    let mut rounds = 0;
    let mut size = 1;
    while size < player_count {
        size *= 2;
        rounds += 1;
    }
    rounds.max(1)
}
