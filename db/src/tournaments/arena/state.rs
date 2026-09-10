use crate::{
    db_error::DbError,
    models::{ArenaGameResult, Game, Tournament, TournamentUser},
    schema::{
        arena_game_results,
        games,
        tournament_final_arena_results,
        tournaments,
        tournaments_users,
    },
    tournaments::rating,
    DbConn,
};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::{Color, GameResult, GameStatus};
use shared_types::{
    tournament::{
        arena::{Config as ArenaConfig, PairingIntent},
        FormatConfig,
    },
    TournamentStatus,
};
use std::{collections::HashMap, ops::Deref, str::FromStr};
use tournamint::{
    arena::{
        ArenaAwardedGameFact,
        ArenaGameAward,
        ArenaPlayerFact,
        ArenaTerminalGameFact,
        ArenaTerminalResult,
    },
    Pairing,
    PlayedGameOutcome,
    PlayerId,
};
use uuid::Uuid;

use super::super::state::{invalid_input, invalid_persisted};

pub(super) struct ArenaDbState {
    pub(super) tournament: Tournament,
    pub(super) configuration: ArenaConfig,
    pub(super) memberships: Vec<ArenaMembership>,
    pub(super) games: Vec<ArenaGame>,
    pub(super) boundaries: ArenaBoundaries,
}

#[derive(Clone)]
pub(super) struct ArenaMembership {
    membership: TournamentUser,
    pub(super) rating: i32,
    pub(super) intent: PairingIntent,
}

impl ArenaMembership {
    pub(super) fn new(membership: TournamentUser, rating: i32, intent: PairingIntent) -> Self {
        Self {
            membership,
            rating,
            intent,
        }
    }

    pub(super) fn record(&self) -> &TournamentUser {
        &self.membership
    }

    pub(super) fn replace_record(&mut self, membership: TournamentUser, intent: PairingIntent) {
        self.membership = membership;
        self.intent = intent;
    }
}

impl Deref for ArenaMembership {
    type Target = TournamentUser;

    fn deref(&self) -> &Self::Target {
        &self.membership
    }
}

#[derive(Clone)]
pub(super) struct ArenaGame {
    pub(super) game: Game,
    pub(super) ordinal: i64,
    pub(super) pairing: Pairing,
    pub(super) terminal: Option<ArenaTerminal>,
    pub(super) award: Option<ArenaGameAward>,
}

#[derive(Clone, Copy)]
pub(super) struct ArenaTerminal {
    pub(super) result: ArenaTerminalResult,
    pub(super) at: DateTime<Utc>,
    pub(super) ratings: [rating::Snapshot; 2],
}

impl ArenaTerminal {
    pub(super) fn from_persisted_game(
        game: &Game,
        pairing: Pairing,
        at: DateTime<Utc>,
        ratings: [rating::Snapshot; 2],
    ) -> Result<Self, DbError> {
        let result = if game.is_arena_no_start() {
            ArenaTerminalResult::NoStart {
                absent: if game.turn == 0 {
                    pairing.white()
                } else {
                    pairing.black()
                },
            }
        } else {
            ArenaTerminalResult::Played(decode_persisted_arena_played_outcome(game)?)
        };
        Ok(Self {
            result,
            at,
            ratings,
        })
    }
}

pub(super) struct ArenaTerminalFact<'a> {
    pub(super) native: ArenaTerminalGameFact,
    pub(super) game: &'a ArenaGame,
    pub(super) ratings: [rating::Snapshot; 2],
}

pub(super) async fn load_arena_state(
    tournament: Tournament,
    configuration: ArenaConfig,
    conn: &mut DbConn<'_>,
) -> Result<ArenaDbState, DbError> {
    if tournament.status() != TournamentStatus::InProgress {
        return Err(DbError::InvalidAction {
            info: String::from("Arena operation requires an in-progress Arena"),
        });
    }
    load_arena_state_for_snapshot(tournament, configuration, conn).await
}

#[cfg(test)]
pub(super) async fn load_arena_state_for_read(
    tournament: Tournament,
    conn: &mut DbConn<'_>,
) -> Result<ArenaDbState, DbError> {
    let configuration = ensure_live_arena(&tournament)?.clone();
    load_arena_state_for_snapshot(tournament, configuration, conn).await
}

pub(super) async fn load_arena_state_for_snapshot(
    tournament: Tournament,
    configuration: ArenaConfig,
    conn: &mut DbConn<'_>,
) -> Result<ArenaDbState, DbError> {
    let memberships = load_arena_memberships(tournament.id, conn).await?;
    let selection = if tournament.status() == TournamentStatus::Finished {
        ArenaGameSelection::Finalized
    } else {
        ArenaGameSelection::All
    };
    let games = load_arena_game_rows(tournament.id, selection, conn).await?;
    let boundaries = load_arena_boundaries(tournament.id, &configuration, conn).await?;
    Ok(ArenaDbState {
        tournament,
        configuration,
        memberships,
        games,
        boundaries,
    })
}

/// The caller holds the tournament lock before this statement starts. Under
/// READ COMMITTED, accepted joins are visible and each concurrent completion's
/// game, award, and participant ratings are either all visible or all absent.
pub(super) async fn load_arena_finalization_snapshot(
    tournament: Tournament,
    configuration: ArenaConfig,
    conn: &mut DbConn<'_>,
) -> Result<ArenaDbState, DbError> {
    let starts_at = tournament
        .starts_at
        .ok_or_else(|| invalid_persisted("Arena tournament is missing its scheduled start"))?;
    let boundaries = arena_boundaries(starts_at, &configuration);
    // Joining games through White emits each game once and retains entrants
    // who have never played. Both player identities come from this same roster.
    let rows = tournaments_users::table
        .left_join(
            games::table.on(games::tournament_id
                .eq(tournaments_users::tournament_id.nullable())
                .and(games::white_id.eq(tournaments_users::user_id))
                .and(games::arena_ordinal.is_not_null())),
        )
        .left_join(arena_game_results::table.on(arena_game_results::game_id.eq(games::id)))
        .filter(tournaments_users::tournament_id.eq(tournament.id))
        .order((
            tournaments_users::pairing_number.asc().nulls_last(),
            tournaments_users::accepted_at.asc(),
            tournaments_users::user_id.asc(),
            games::arena_ordinal.asc(),
        ))
        .select((
            TournamentUser::as_select(),
            Option::<Game>::as_select(),
            Option::<ArenaGameResult>::as_select(),
        ))
        .load::<(TournamentUser, Option<Game>, Option<ArenaGameResult>)>(conn)
        .await?;
    let mut memberships = Vec::<ArenaMembership>::new();
    let mut persisted_games = Vec::new();
    for (membership, game, result) in rows {
        if memberships
            .last()
            .is_none_or(|last| last.user_id != membership.user_id)
        {
            memberships.push(decode_arena_membership(membership)?);
        }
        if let Some(game) = game {
            persisted_games.push((game, result));
        }
    }
    let pairing_numbers = memberships
        .iter()
        .map(|membership| {
            membership
                .pairing_number
                .map(|number| (membership.user_id, number))
                .ok_or_else(|| invalid_persisted("Arena participant has no pairing number"))
        })
        .collect::<Result<HashMap<_, _>, DbError>>()?;
    let mut games = persisted_games
        .into_iter()
        .map(|(game, result)| {
            let number = |user_id| {
                pairing_numbers.get(&user_id).copied().ok_or_else(|| {
                    invalid_persisted("Arena Game player is missing from its roster snapshot")
                })
            };
            let white = number(game.white_id)?;
            let black = number(game.black_id)?;
            decode_arena_game(game, white, black, result)
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    games.sort_by_key(|game| game.ordinal);
    Ok(ArenaDbState {
        tournament,
        configuration,
        memberships,
        games,
        boundaries,
    })
}

pub(super) async fn load_arena_memberships(
    tournament_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<Vec<ArenaMembership>, DbError> {
    tournaments_users::table
        .filter(tournaments_users::tournament_id.eq(tournament_id))
        .order((
            tournaments_users::pairing_number.asc().nulls_last(),
            tournaments_users::accepted_at.asc(),
            tournaments_users::user_id.asc(),
        ))
        .select(TournamentUser::as_select())
        .load::<TournamentUser>(conn)
        .await?
        .into_iter()
        .map(decode_arena_membership)
        .collect::<Result<Vec<_>, DbError>>()
}

fn decode_arena_membership(membership: TournamentUser) -> Result<ArenaMembership, DbError> {
    let rating = membership
        .arena_rating
        .ok_or_else(|| invalid_persisted("Arena participant has no rating"))?;
    let intent = membership
        .arena_pairing_intent
        .as_deref()
        .ok_or_else(|| invalid_persisted("Arena participant has no pairing intent"))?;
    let intent = decode_arena_pairing_intent(intent)?;
    Ok(ArenaMembership::new(membership, rating, intent))
}

pub(super) fn decode_arena_pairing_intent(value: &str) -> Result<PairingIntent, DbError> {
    match value {
        "enabled" => Ok(PairingIntent::Enabled),
        "paused" => Ok(PairingIntent::Paused),
        found => Err(invalid_persisted(&format!(
            "Arena participant has invalid pairing intent: {found}"
        ))),
    }
}

enum ArenaGameSelection {
    All,
    CompletedFor([Uuid; 2]),
    Finalized,
}

pub(super) async fn load_arena_scoring_games(
    tournament_id: Uuid,
    players: [Uuid; 2],
    conn: &mut DbConn<'_>,
) -> Result<Vec<ArenaGame>, DbError> {
    load_arena_game_rows(
        tournament_id,
        ArenaGameSelection::CompletedFor(players),
        conn,
    )
    .await
}

async fn load_arena_game_rows(
    tournament_id: Uuid,
    selection: ArenaGameSelection,
    conn: &mut DbConn<'_>,
) -> Result<Vec<ArenaGame>, DbError> {
    let (white_membership, black_membership) = diesel::alias!(
        crate::schema::tournaments_users as arena_game_white,
        crate::schema::tournaments_users as arena_game_black,
    );
    let mut query = games::table
        .left_join(arena_game_results::table.on(arena_game_results::game_id.eq(games::id)))
        .inner_join(
            white_membership.on(games::tournament_id
                .eq(white_membership
                    .field(tournaments_users::tournament_id)
                    .nullable())
                .and(games::white_id.eq(white_membership.field(tournaments_users::user_id)))),
        )
        .inner_join(
            black_membership.on(games::tournament_id
                .eq(black_membership
                    .field(tournaments_users::tournament_id)
                    .nullable())
                .and(games::black_id.eq(black_membership.field(tournaments_users::user_id)))),
        )
        .filter(games::tournament_id.eq(Some(tournament_id)))
        .select((
            Game::as_select(),
            white_membership
                .field(tournaments_users::pairing_number)
                .assume_not_null(),
            black_membership
                .field(tournaments_users::pairing_number)
                .assume_not_null(),
            Option::<ArenaGameResult>::as_select(),
        ))
        .order(games::arena_ordinal.asc())
        .into_boxed();
    match selection {
        ArenaGameSelection::All => {
            query = query.filter(games::arena_ordinal.is_not_null());
        }
        ArenaGameSelection::CompletedFor(players) => {
            query = query
                .filter(games::arena_ordinal.is_not_null())
                .filter(games::finished.eq(true))
                .filter(
                    games::white_id
                        .eq_any(players)
                        .or(games::black_id.eq_any(players)),
                );
        }
        ArenaGameSelection::Finalized => {
            query = query.filter(
                games::id.eq_any(
                    tournament_final_arena_results::table
                        .filter(tournament_final_arena_results::tournament_id.eq(tournament_id))
                        .select(tournament_final_arena_results::game_id),
                ),
            );
        }
    }
    let persisted = query
        .load::<(Game, i32, i32, Option<ArenaGameResult>)>(conn)
        .await?;

    persisted
        .into_iter()
        .map(|(game, white, black, result)| decode_arena_game(game, white, black, result))
        .collect::<Result<Vec<_>, DbError>>()
}

fn decode_arena_game(
    game: Game,
    white: i32,
    black: i32,
    result: Option<ArenaGameResult>,
) -> Result<ArenaGame, DbError> {
    let player_id = |pairing_number| {
        usize::try_from(pairing_number)
            .map(PlayerId::new)
            .map_err(|_| invalid_persisted("Arena pairing number is negative"))
    };
    let ordinal = game
        .arena_ordinal
        .ok_or_else(|| invalid_persisted("Arena Game is missing its ordinal"))?;
    let pairing = Pairing::new(player_id(white)?, player_id(black)?);
    let terminal = if game.finished {
        let terminal_at = game.finished_at.ok_or_else(|| {
            invalid_persisted("Arena terminal is missing its completion timestamp")
        })?;
        let ratings =
            [
                rating::snapshot(game.white_rating.ok_or_else(|| {
                    invalid_persisted("Arena terminal is missing its White rating")
                })?)
                .ok_or_else(|| invalid_persisted("Arena terminal has an invalid White rating"))?,
                rating::snapshot(game.black_rating.ok_or_else(|| {
                    invalid_persisted("Arena terminal is missing its Black rating")
                })?)
                .ok_or_else(|| invalid_persisted("Arena terminal has an invalid Black rating"))?,
            ];
        Some(ArenaTerminal::from_persisted_game(
            &game,
            pairing,
            terminal_at,
            ratings,
        )?)
    } else {
        None
    };
    Ok(ArenaGame {
        game,
        ordinal,
        pairing,
        terminal,
        award: result.map(|result| result.award()).transpose()?,
    })
}

pub(super) fn facts_before(
    state: &ArenaDbState,
    observed_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
) -> Result<(Vec<ArenaPlayerFact>, Vec<ArenaTerminalFact<'_>>), DbError> {
    let players = state
        .memberships
        .iter()
        .enumerate()
        .map(|(index, membership)| {
            let waiting_millis = membership
                .arena_waiting_since
                .map(|waiting_since| waiting_millis(observed_at, waiting_since))
                .transpose()?;
            Ok(ArenaPlayerFact {
                player: PlayerId::new(index),
                arena_rating: Some(membership.rating),
                waiting_millis,
            })
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    Ok((players, terminal_facts_before(&state.games, ends_at)?))
}

pub(super) fn terminal_facts_before(
    games: &[ArenaGame],
    ends_at: DateTime<Utc>,
) -> Result<Vec<ArenaTerminalFact<'_>>, DbError> {
    collect_terminal_facts(
        games
            .iter()
            .filter(|game| game.terminal.is_some_and(|terminal| terminal.at < ends_at)),
    )
}

pub(super) fn terminal_facts(games: &[ArenaGame]) -> Result<Vec<ArenaTerminalFact<'_>>, DbError> {
    collect_terminal_facts(games.iter())
}

fn collect_terminal_facts<'a>(
    games: impl Iterator<Item = &'a ArenaGame>,
) -> Result<Vec<ArenaTerminalFact<'a>>, DbError> {
    let mut terminals = Vec::new();
    for arena_game in games {
        let Some(terminal) = arena_game.terminal else {
            continue;
        };
        let game = &arena_game.game;
        terminals.push(ArenaTerminalFact {
            native: ArenaTerminalGameFact {
                pairing: arena_game.pairing,
                result: terminal.result,
                plies: u32::try_from(game.turn)
                    .map_err(|_| invalid_persisted("Arena game has a negative ply count"))?,
                white_berserked: game.white_berserked,
                black_berserked: game.black_berserked,
                white_rating: Some(rating::native(terminal.ratings[0])),
                black_rating: Some(rating::native(terminal.ratings[1])),
            },
            game: arena_game,
            ratings: terminal.ratings,
        });
    }
    Ok(terminals)
}

pub(super) fn awarded_facts(
    terminals: &[ArenaTerminalFact<'_>],
) -> Result<Vec<ArenaAwardedGameFact>, DbError> {
    terminals
        .iter()
        .map(|terminal| {
            let award = terminal.game.award.ok_or_else(|| {
                invalid_persisted("Arena terminal is missing its persisted award")
            })?;
            Ok(ArenaAwardedGameFact {
                game: terminal.native,
                award,
            })
        })
        .collect()
}

pub(super) fn projection_players(state: &ArenaDbState) -> Vec<ArenaPlayerFact> {
    state
        .memberships
        .iter()
        .enumerate()
        .map(|(index, membership)| ArenaPlayerFact {
            player: PlayerId::new(index),
            arena_rating: Some(membership.rating),
            waiting_millis: None,
        })
        .collect()
}

pub(super) fn arena_configuration(tournament: &Tournament) -> Result<&ArenaConfig, DbError> {
    match &tournament.configuration().format {
        FormatConfig::Arena(config) => Ok(config),
        _ => Err(invalid_input("The Arena adapter received another format")),
    }
}
pub(super) fn ensure_live_arena(tournament: &Tournament) -> Result<&ArenaConfig, DbError> {
    let config = arena_configuration(tournament)?;
    if tournament.status() != TournamentStatus::InProgress {
        return Err(DbError::InvalidAction {
            info: String::from("Arena operation requires an in-progress Arena"),
        });
    }
    Ok(config)
}
pub(super) fn arena_config(state: &ArenaDbState) -> &ArenaConfig {
    &state.configuration
}

#[derive(Clone, Copy)]
pub(super) struct ArenaBoundaries {
    pub(super) starts_at: DateTime<Utc>,
    pub(super) pairing_closes_at: DateTime<Utc>,
    pub(super) ends_at: DateTime<Utc>,
}

pub(super) async fn load_arena_boundaries(
    tournament_id: Uuid,
    config: &ArenaConfig,
    conn: &mut DbConn<'_>,
) -> Result<ArenaBoundaries, DbError> {
    let starts = tournaments::table
        .find(tournament_id)
        .select(tournaments::starts_at.assume_not_null())
        .first::<DateTime<Utc>>(conn)
        .await?;
    Ok(arena_boundaries(starts, config))
}

fn arena_boundaries(starts: DateTime<Utc>, config: &ArenaConfig) -> ArenaBoundaries {
    let ends = starts + Duration::seconds(i64::from(config.duration_seconds.get()));
    let closes = ends - Duration::seconds(60);
    ArenaBoundaries {
        starts_at: starts,
        pairing_closes_at: closes,
        ends_at: ends,
    }
}
pub(super) fn active_game_map(games: &[ArenaGame]) -> HashMap<Uuid, Uuid> {
    let mut map = HashMap::new();
    for arena_game in games {
        if arena_game.terminal.is_none() {
            map.insert(arena_game.game.white_id, arena_game.game.id);
            map.insert(arena_game.game.black_id, arena_game.game.id);
        }
    }
    map
}
pub(super) fn waiting_millis(
    observed_at: DateTime<Utc>,
    waiting_since: DateTime<Utc>,
) -> Result<u64, DbError> {
    u64::try_from(
        observed_at
            .signed_duration_since(waiting_since)
            .num_milliseconds(),
    )
    .map_err(|_| DbError::SerializationConflict)
}
fn decode_persisted_arena_played_outcome(game: &Game) -> Result<PlayedGameOutcome, DbError> {
    match GameStatus::from_str(&game.game_status)
        .map_err(|e| invalid_persisted(&format!("Arena terminal has invalid status: {e}")))?
    {
        GameStatus::Finished(GameResult::Winner(Color::White)) => Ok(PlayedGameOutcome::WhiteWin),
        GameStatus::Finished(GameResult::Draw) => Ok(PlayedGameOutcome::Draw),
        GameStatus::Finished(GameResult::Winner(Color::Black)) => Ok(PlayedGameOutcome::BlackWin),
        status => Err(invalid_persisted(&format!(
            "Arena terminal status cannot produce a played outcome: {status}"
        ))),
    }
}
