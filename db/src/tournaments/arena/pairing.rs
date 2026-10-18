use crate::{
    db_error::DbError,
    models::{Game, NewGame, Rating, Tournament, TournamentUser, User},
    schema::{tournaments, tournaments_invitations, tournaments_users, users},
    tournaments::rating,
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{delete, prelude::*, update};
use diesel_async::{AnsiTransactionManager, AsyncConnection, RunQueryDsl, TransactionManager};
use shared_types::{
    tournament::{
        arena::{Config as ArenaConfig, PairingIntent},
        Clock,
    },
    GameSpeed,
    TournamentStatus,
};
use std::collections::HashSet;
use tournamint::{arena, Pairing};
use uuid::Uuid;

use super::{
    super::{
        arena::{
            JoinOutcome as ArenaJoinOutcome,
            PairingOutcome as ArenaPairingOutcome,
            StartOutcome as ArenaStartOutcome,
        },
        state::invalid_persisted,
    },
    settlement::sample_arena_ratings,
    state::{
        arena_config,
        arena_configuration,
        awarded_facts,
        decode_locked_arena_memberships,
        ensure_live_arena,
        facts_before,
        load_arena_active_players,
        load_arena_boundaries,
        load_arena_state,
        ArenaDbState,
        ArenaGame,
        ArenaMembership,
    },
};

pub async fn start_scheduled_with_presence<P>(
    tournament_id: Uuid,
    presence: P,
    conn: &mut DbConn<'_>,
) -> Result<ArenaStartOutcome, DbError>
where
    P: Fn(&[Uuid]) -> HashSet<Uuid> + Send + Sync + 'static,
{
    conn.transaction::<_, DbError, _>(async move |tc| {
        let tournament = Tournament::find_for_update(tournament_id, tc).await?;
        let observed_at = Utc::now();
        let configuration = arena_configuration(&tournament)?.clone();
        start(tournament, configuration, observed_at, &presence, tc).await
    })
    .await
}

async fn start<P>(
    tournament: Tournament,
    configuration: ArenaConfig,
    observed_at: DateTime<Utc>,
    presence: &P,
    conn: &mut DbConn<'_>,
) -> Result<ArenaStartOutcome, DbError>
where
    P: Fn(&[Uuid]) -> HashSet<Uuid> + Send + Sync,
{
    match tournament.status() {
        TournamentStatus::InProgress | TournamentStatus::Finished => Ok(ArenaStartOutcome {
            tournament,
            started_now: false,
            new_games: Vec::new(),
            removed_invitees: Vec::new(),
        }),
        TournamentStatus::NotStarted => {
            start_new_arena(tournament, configuration, observed_at, presence, conn).await
        }
    }
}

async fn start_new_arena<P>(
    tournament: Tournament,
    arena_configuration: ArenaConfig,
    observed_at: DateTime<Utc>,
    presence: &P,
    conn: &mut DbConn<'_>,
) -> Result<ArenaStartOutcome, DbError>
where
    P: Fn(&[Uuid]) -> HashSet<Uuid> + Send + Sync,
{
    let boundaries = load_arena_boundaries(&tournament, &arena_configuration)?;
    if observed_at < boundaries.starts_at {
        return Err(DbError::InvalidAction {
            info: String::from("The scheduled Arena start is not due"),
        });
    }
    let pairing_open = observed_at < boundaries.pairing_closes_at;
    let mut memberships =
        TournamentUser::find_by_tournament_id_for_update(tournament.id, conn).await?;
    let user_ids = memberships.iter().map(|m| m.user_id).collect::<Vec<_>>();
    User::ensure_active_ids(&user_ids, conn).await?;
    let mut ratings = sample_arena_ratings(
        &user_ids,
        GameSpeed::from(Clock::Realtime(arena_configuration.game_clock)),
        conn,
    )
    .await?;
    let online = if pairing_open {
        presence(&user_ids)
    } else {
        Default::default()
    };
    for (index, membership) in memberships.iter_mut().enumerate() {
        let rating = ratings
            .remove(&membership.user_id)
            .ok_or_else(|| invalid_persisted("Arena entrant is missing its sampled rating"))?;
        let pairing_number = i32::try_from(index)
            .map_err(|_| invalid_persisted("Arena pairing number exceeds the storage domain"))?;
        *membership = update(tournaments_users::table.find((tournament.id, membership.user_id)))
            .set((
                tournaments_users::pairing_number.eq(pairing_number),
                tournaments_users::arena_rating.eq(rating),
                tournaments_users::arena_pairing_intent.eq("enabled"),
                tournaments_users::arena_waiting_since
                    .eq((pairing_open && online.contains(&membership.user_id))
                        .then_some(observed_at)),
            ))
            .get_result(conn)
            .await?;
    }
    let tournament = Tournament::persist_started(&tournament, observed_at, conn).await?;
    let removed_invitees = tournaments_invitations::table
        .filter(tournaments_invitations::tournament_id.eq(tournament.id))
        .select(tournaments_invitations::invitee_id)
        .load::<Uuid>(conn)
        .await?;
    delete(
        tournaments_invitations::table
            .filter(tournaments_invitations::tournament_id.eq(tournament.id)),
    )
    .execute(conn)
    .await?;
    let memberships = decode_locked_arena_memberships(memberships)?;
    let mut state = load_arena_state(tournament, arena_configuration, memberships, conn).await?;
    let new_games = if pairing_open {
        pair_and_materialize(&mut state, observed_at, &online, conn).await?
    } else {
        Vec::new()
    };
    Ok(ArenaStartOutcome {
        tournament: state.tournament,
        started_now: true,
        new_games,
        removed_invitees,
    })
}

pub async fn join_with_presence<P>(
    tournament_id: Uuid,
    user_id: Uuid,
    presence: P,
    conn: &mut DbConn<'_>,
) -> Result<ArenaJoinOutcome, DbError>
where
    P: Fn(&[Uuid]) -> HashSet<Uuid> + Send + Sync + 'static,
{
    conn.transaction::<_, DbError, _>(async move |tc| {
        join_with_presence_in_transaction(tournament_id, user_id, &presence, tc).await
    })
    .await
}

/// The caller owns a transaction and must roll back on error.
/// The tournament lock serializes admission and pairing-number allocation.
pub async fn join_with_presence_in_transaction<P>(
    tournament_id: Uuid,
    user_id: Uuid,
    presence: &P,
    tc: &mut DbConn<'_>,
) -> Result<ArenaJoinOutcome, DbError>
where
    P: Fn(&[Uuid]) -> HashSet<Uuid> + Send + Sync,
{
    if AnsiTransactionManager::transaction_manager_status_mut(&mut **tc)
        .transaction_depth()?
        .is_none()
    {
        return Err(diesel::result::Error::NotInTransaction.into());
    }
    // New entrants have no membership row to lock. Lock the tournament first
    // so pairing-number allocation and insertion serialize with other joins.
    let tournament = Tournament::find_for_update(tournament_id, tc).await?;
    let configuration = ensure_live_arena(&tournament)?.clone();
    tournament.ensure_not_invite_only(&user_id, tc).await?;
    let existing = TournamentUser::find_for_update(tournament_id, user_id, tc).await?;
    let observed_at = Utc::now();
    let boundaries = load_arena_boundaries(&tournament, &configuration)?;
    if observed_at >= boundaries.pairing_closes_at {
        return Err(DbError::InvalidAction {
            info: String::from("Arena Join is closed at the pairing cutoff"),
        });
    }
    User::ensure_active_ids(&[user_id], tc).await?;
    let online = presence(&[user_id]).contains(&user_id);
    let joined_now = existing.is_none();
    let membership = match existing {
        Some(membership) => membership,
        None => {
            let user = User::find_active_by_uuid(&user_id, tc).await?;
            tournament.ensure_user_bot_admission(&user)?;
            let rating = rating::rounded(
                Rating::for_uuid(
                    &user_id,
                    &GameSpeed::from(Clock::Realtime(configuration.game_clock)),
                    tc,
                )
                .await?
                .rating,
            )
            .ok_or_else(|| invalid_persisted("Arena entrant has an invalid rating"))?;
            tournament.ensure_rating_in_band(f64::from(rating))?;
            let pairing_number =
                TournamentUser::next_arena_pairing_number(tournament.id, tc).await?;
            TournamentUser::insert_live_arena(
                tournament.id,
                user_id,
                observed_at,
                pairing_number,
                rating,
                online.then_some(observed_at),
                tc,
            )
            .await?
        }
    };
    if joined_now {
        return Ok(ArenaJoinOutcome { joined_now });
    }
    let active = tournament.has_active_game_for(user_id, tc).await?;
    let waiting_since =
        (online && !active).then_some(membership.arena_waiting_since.unwrap_or(observed_at));
    TournamentUser::persist_arena_pairing_state(
        tournament.id,
        user_id,
        PairingIntent::Enabled,
        waiting_since,
        tc,
    )
    .await?;
    Ok(ArenaJoinOutcome { joined_now })
}

pub async fn pair_waiting_with_presence<P>(
    tournament_id: Uuid,
    presence: P,
    conn: &mut DbConn<'_>,
) -> Result<ArenaPairingOutcome, DbError>
where
    P: Fn(&[Uuid]) -> HashSet<Uuid> + Send + Sync,
{
    conn.transaction::<_, DbError, _>(async move |tc| {
        let tournament = Tournament::find_for_update(tournament_id, tc).await?;
        let configuration = arena_configuration(&tournament)?.clone();
        match tournament.status() {
            TournamentStatus::NotStarted => Err(DbError::InvalidAction {
                info: String::from("Arena tournament has not started"),
            }),
            TournamentStatus::Finished => Ok(ArenaPairingOutcome {
                changed: false,
                new_games: Vec::new(),
            }),
            TournamentStatus::InProgress => {
                let boundaries = load_arena_boundaries(&tournament, &configuration)?;
                let memberships =
                    TournamentUser::find_by_tournament_id_for_update(tournament.id, tc).await?;
                let observed_at = Utc::now();
                if observed_at >= boundaries.pairing_closes_at {
                    return Ok(ArenaPairingOutcome {
                        changed: false,
                        new_games: Vec::new(),
                    });
                }
                let mut memberships = decode_locked_arena_memberships(memberships)?;
                let active = load_arena_active_players(tournament.id, tc).await?;
                let ids = memberships.iter().map(|m| m.user_id).collect::<Vec<_>>();
                let active_users = users::table
                    .filter(users::id.eq_any(&ids))
                    .filter(users::deleted.eq(false))
                    .select(users::id)
                    .load::<Uuid>(tc)
                    .await?
                    .into_iter()
                    .collect::<HashSet<_>>();
                let online = presence(&ids)
                    .intersection(&active_users)
                    .copied()
                    .collect();
                let mut changed = reconcile_presence(
                    tournament.id,
                    &mut memberships,
                    &active,
                    &online,
                    observed_at,
                    tc,
                )
                .await?;
                // An idle sweep still reconciles waiting timestamps, but needs no
                // completed-game history until at least two players can pair.
                if memberships
                    .iter()
                    .filter(|membership| membership.arena_waiting_since.is_some())
                    .take(2)
                    .count()
                    < 2
                {
                    return Ok(ArenaPairingOutcome {
                        changed,
                        new_games: Vec::new(),
                    });
                }
                let active_entrants = online
                    .union(&active)
                    .filter(|user_id| active_users.contains(*user_id))
                    .copied()
                    .collect();
                let mut state =
                    load_arena_state(tournament, configuration, memberships, tc).await?;
                let new_games =
                    pair_and_materialize(&mut state, observed_at, &active_entrants, tc).await?;
                changed |= !new_games.is_empty();
                Ok(ArenaPairingOutcome { changed, new_games })
            }
        }
    })
    .await
}

pub async fn set_pairing_intent(
    tournament_id: Uuid,
    user_id: Uuid,
    intent: PairingIntent,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    conn.transaction::<_, DbError, _>(async move |tc| {
        User::ensure_active_ids(&[user_id], tc).await?;
        let membership = TournamentUser::find_for_update(tournament_id, user_id, tc)
            .await?
            .ok_or_else(|| DbError::InvalidAction {
                info: String::from("User is not an Arena participant"),
            })?;
        let observed_at = Utc::now();
        let tournament = Tournament::find(tournament_id, tc).await?;
        arena_configuration(&tournament)?;
        // A post-game Pause may arrive after the Arena has finished.
        if intent == PairingIntent::Paused && tournament.status() == TournamentStatus::Finished {
            return Ok(false);
        }
        let config = ensure_live_arena(&tournament)?;
        // A post-game pause can repeat the automatic pause after a no-start.
        if intent == PairingIntent::Paused && membership.pairing_intent()? == Some(intent) {
            return Ok(false);
        }
        let pairing_closes_at = load_arena_boundaries(&tournament, config)?.pairing_closes_at;
        let active = tournament.has_active_game_for(user_id, tc).await?;
        let (expected, waiting_since) = match intent {
            PairingIntent::Paused => (PairingIntent::Enabled, None),
            PairingIntent::Enabled => {
                if observed_at >= pairing_closes_at {
                    return Err(DbError::InvalidAction {
                        info: String::from("Arena Resume is closed at the pairing cutoff"),
                    });
                }
                (PairingIntent::Paused, (!active).then_some(observed_at))
            }
        };
        if membership.pairing_intent()? != Some(expected) {
            return Err(DbError::InvalidAction {
                info: String::from("Arena entrant already has the requested pairing intent"),
            });
        }
        TournamentUser::persist_arena_pairing_state(
            tournament_id,
            user_id,
            intent,
            waiting_since,
            tc,
        )
        .await?;
        // Only Resume must roll back if finalization committed during the update.
        if intent == PairingIntent::Enabled {
            let rechecked = Tournament::find(tournament_id, tc).await?;
            ensure_live_arena(&rechecked)?;
        }
        Ok(true)
    })
    .await
}

async fn reconcile_presence(
    tournament_id: Uuid,
    memberships: &mut [ArenaMembership],
    active: &HashSet<Uuid>,
    online: &HashSet<Uuid>,
    observed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    let mut changed = false;
    for membership in memberships {
        let enabled =
            membership.intent == PairingIntent::Enabled && membership.withdrawn_at.is_none();
        let next_waiting = (enabled
            && online.contains(&membership.user_id)
            && !active.contains(&membership.user_id))
        .then_some(membership.arena_waiting_since.unwrap_or(observed_at));
        if membership.arena_waiting_since != next_waiting {
            let intent = membership.intent;
            let persisted = TournamentUser::persist_arena_pairing_state(
                tournament_id,
                membership.user_id,
                intent,
                next_waiting,
                conn,
            )
            .await?;
            membership.replace_record(persisted, intent)?;
            changed = true;
        }
    }
    Ok(changed)
}

async fn pair_and_materialize(
    state: &mut ArenaDbState,
    observed_at: DateTime<Utc>,
    active_entrants: &HashSet<Uuid>,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Game>, DbError> {
    if observed_at >= state.boundaries.pairing_closes_at {
        return Ok(Vec::new());
    }
    let (mut players, terminals) = facts_before(state, observed_at, state.boundaries.ends_at)?;
    for (player, membership) in players.iter_mut().zip(&state.memberships) {
        // Unfinished games still block duels; offline idle entrants do not.
        player.active &= active_entrants.contains(&membership.user_id);
    }
    let native_terminals = awarded_facts(&terminals)?;
    let config = &state.configuration.rules;
    let selected = arena::pair(config, &players, &native_terminals)
        .map_err(|e| invalid_persisted(&format!("Arena pairing facts are invalid: {e}")))?;
    let (materialized, new_games) =
        materialize_pairings(state, observed_at, &selected.pairings, conn).await?;
    if let Some((first, remaining)) = materialized.split_first() {
        persist_featured_game_after_wave(
            state,
            first,
            remaining,
            &selected.ranks,
            observed_at,
            conn,
        )
        .await?;
    }
    state.games.extend(materialized);
    Ok(new_games)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RankedFeaturedGame {
    pub(super) game_id: Uuid,
    pub(super) priority: u32,
    pub(super) ordinal: i64,
}

pub(super) fn choose_featured_game(
    current: Option<(Uuid, u32, bool)>,
    first: RankedFeaturedGame,
    remaining: &[RankedFeaturedGame],
) -> Uuid {
    let best = remaining.iter().fold(first, |best, candidate| {
        if (candidate.priority, candidate.ordinal) < (best.priority, best.ordinal) {
            *candidate
        } else {
            best
        }
    });
    match current {
        Some((game_id, priority, true)) if priority <= best.priority => game_id,
        Some(_) | None => best.game_id,
    }
}

fn featured_priority(game: &ArenaGame, ranks: &[u32]) -> u32 {
    featured_priority_for_players(game.pairing, ranks)
}

pub(super) fn featured_priority_for_players(pairing: Pairing, ranks: &[u32]) -> u32 {
    ranks[pairing.white().index()].max(ranks[pairing.black().index()])
}

async fn persist_featured_game_after_wave(
    state: &mut ArenaDbState,
    first_game: &ArenaGame,
    remaining_games: &[ArenaGame],
    ranks: &[u32],
    observed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    let ranked = |game: &ArenaGame| RankedFeaturedGame {
        game_id: game.game.id,
        priority: featured_priority(game, ranks),
        ordinal: game.ordinal,
    };
    let first = ranked(first_game);
    let remaining = remaining_games.iter().map(ranked).collect::<Vec<_>>();
    let current = state
        .tournament
        .featured_game_id
        .and_then(|featured_game_id| {
            state
                .games
                .iter()
                .find(|game| game.game.id == featured_game_id)
                .map(|game| {
                    (
                        game.game.id,
                        featured_priority(game, ranks),
                        game.terminal.is_none(),
                    )
                })
        });
    let featured_game_id = choose_featured_game(current, first, &remaining);
    if state.tournament.featured_game_id == Some(featured_game_id) {
        return Ok(());
    }
    state.tournament = update(tournaments::table.find(state.tournament.id))
        .set((
            tournaments::featured_game_id.eq(Some(featured_game_id)),
            tournaments::updated_at.eq(observed_at),
        ))
        .get_result(conn)
        .await?;
    Ok(())
}

async fn materialize_pairings(
    state: &mut ArenaDbState,
    paired_at: DateTime<Utc>,
    selected: &[Pairing],
    conn: &mut DbConn<'_>,
) -> Result<(Vec<ArenaGame>, Vec<Game>), DbError> {
    let config = arena_config(state).clone();
    let mut games = Vec::new();
    let mut new_games = Vec::new();
    let mut paired_players = HashSet::new();
    let next_ordinal =
        i64::try_from(state.games.len()).expect("Arena game count fits the storage ordinal domain");
    for pairing in selected {
        let white = state.memberships[pairing.white().index()].user_id;
        let black = state.memberships[pairing.black().index()].user_id;
        paired_players.insert(white);
        paired_players.insert(black);
        let ordinal =
            next_ordinal + i64::try_from(games.len()).expect("Arena pairing wave size fits i64");
        let game = Game::create(
            NewGame::for_arena(
                state.tournament.id,
                ordinal,
                white,
                black,
                config.game_clock,
                paired_at,
            )?,
            conn,
        )
        .await?;
        games.push(ArenaGame {
            game: (&game).into(),
            ordinal,
            pairing: *pairing,
            terminal: None,
            award: None,
        });
        new_games.push(game);
    }
    for membership in &mut state.memberships {
        if paired_players.contains(&membership.user_id) {
            let persisted = TournamentUser::persist_arena_pairing_state(
                state.tournament.id,
                membership.user_id,
                PairingIntent::Enabled,
                None,
                conn,
            )
            .await?;
            membership.replace_record(persisted, PairingIntent::Enabled)?;
        }
    }
    Ok((games, new_games))
}
