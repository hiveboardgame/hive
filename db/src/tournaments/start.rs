use crate::{
    db_error::DbError,
    helpers::run_serializable,
    models::{
        Game,
        Rating,
        Tournament,
        TournamentEliminationNode,
        TournamentSlot,
        TournamentSlotInsert,
        TournamentUser,
        User,
    },
    schema::{tournaments, tournaments_invitations},
    tournaments::rating,
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use shared_types::{
    tournament::{Config, FormatConfig, SlotKey, MIN_SWISS_START_SEATS},
    Clock,
    GameSpeed,
    TournamentStartSetup,
    TournamentStatus,
};
use std::collections::HashSet;
use tournamint::elimination::EliminationNodeFact;
use uuid::Uuid;

use super::{
    progress_swiss,
    state::{invalid_input, load_in_progress, BotUsers},
    transition::release_slot_into_games,
};

fn validate_roster(expected: &[Uuid], actual: &[Uuid]) -> Result<(), DbError> {
    let expected_set = expected.iter().copied().collect::<HashSet<_>>();
    let actual_set = actual.iter().copied().collect::<HashSet<_>>();
    if expected.len() != actual.len()
        || actual_set.len() != actual.len()
        || expected_set != actual_set
    {
        return Err(invalid_input(
            "Bracket placement must contain every frozen entrant exactly once",
        ));
    }
    Ok(())
}

fn owned_setup(
    tournament: &Tournament,
    setup_id: Uuid,
    owner_id: Uuid,
    now: DateTime<Utc>,
) -> Result<TournamentStartSetup, DbError> {
    let setup = tournament
        .start_setup()
        .ok_or_else(|| invalid_input("There is no bracket setup to control"))?;
    if setup.id != setup_id || setup.owner_id != owner_id {
        return Err(invalid_input(
            "Only the person who opened this bracket setup can control it",
        ));
    }
    if !setup.active_at(now) {
        return Err(invalid_input("The bracket setup expired after 15 minutes"));
    }
    Ok(setup)
}

pub async fn prepare_elimination_start(
    tournament_id: Uuid,
    owner_id: Uuid,
    expected_entrant_ids: Vec<Uuid>,
    conn: &mut DbConn<'_>,
) -> Result<Tournament, DbError> {
    run_serializable(conn, move |tc| {
        let expected_entrant_ids = expected_entrant_ids.clone();
        Box::pin(prepare_elimination_start_in_transaction(
            tournament_id,
            owner_id,
            expected_entrant_ids,
            tc,
        ))
    })
    .await
}

/// Prepares a bracket inside a caller-owned SERIALIZABLE transaction.
/// Ordinary commands use `prepare_elimination_start`.
pub async fn prepare_elimination_start_in_transaction(
    tournament_id: Uuid,
    owner_id: Uuid,
    expected_entrant_ids: Vec<Uuid>,
    tc: &mut DbConn<'_>,
) -> Result<Tournament, DbError> {
    let tournament = Tournament::find_for_update(tournament_id, tc).await?;
    let now = Utc::now();
    authorize_start(&tournament, StartTrigger::Organizer(owner_id), now, tc).await?;
    if !matches!(
        tournament.configuration().format,
        FormatConfig::Elimination(_)
    ) {
        return Err(invalid_input(
            "Bracket setup is only available for elimination tournaments",
        ));
    }
    if tournament
        .start_setup()
        .is_some_and(|setup| setup.active_at(now))
    {
        return Err(invalid_input("Another bracket setup is already active"));
    }
    let memberships = TournamentUser::find_by_tournament_id(tournament_id, tc).await?;
    let actual = memberships
        .iter()
        .map(|row| row.user_id)
        .collect::<Vec<_>>();
    validate_roster(&actual, &expected_entrant_ids)?;
    if memberships.len() < usize::try_from(tournament.min_seats).unwrap_or(usize::MAX) {
        return Err(DbError::NotEnoughPlayers);
    }
    User::tournament_bot_ids(&actual, tc).await?;
    let prepared =
        PreparedFixedFieldStart::new(tournament.configuration(), memberships.len(), None)?;
    let seeded = freeze_fixed_field(prepared.rating_clock(), memberships, tc).await?;
    let setup = TournamentStartSetup {
        id: Uuid::new_v4(),
        owner_id,
        opened_at: now,
        seeded_players: seeded.into_iter().map(|row| row.user_id).collect(),
    };
    Ok(diesel::update(tournaments::table.find(tournament_id))
        .set((
            tournaments::start_setup.eq(Some(
                serde_json::to_value(setup).expect("serializable bracket setup"),
            )),
            tournaments::updated_at.eq(now),
        ))
        .get_result(tc)
        .await?)
}

pub async fn cancel_elimination_start(
    tournament_id: Uuid,
    owner_id: Uuid,
    setup_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<Tournament, DbError> {
    conn.transaction::<_, DbError, _>(async move |tc| {
        let tournament = Tournament::find_for_update(tournament_id, tc).await?;
        owned_setup(&tournament, setup_id, owner_id, Utc::now())?;
        Ok(diesel::update(tournaments::table.find(tournament_id))
            .set((
                tournaments::start_setup.eq(None::<serde_json::Value>),
                tournaments::updated_at.eq(Utc::now()),
            ))
            .get_result(tc)
            .await?)
    })
    .await
}

pub async fn confirm_elimination_start(
    tournament_id: Uuid,
    owner_id: Uuid,
    setup_id: Uuid,
    bracket_order: Vec<Uuid>,
    conn: &mut DbConn<'_>,
) -> Result<StartOutcome, DbError> {
    start_fixed_field(
        tournament_id,
        StartTrigger::Organizer(owner_id),
        None,
        Some((setup_id, bracket_order)),
        conn,
    )
    .await
}

pub async fn expire_elimination_setups(
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Tournament>, DbError> {
    let candidates = tournaments::table
        .filter(tournaments::start_setup.is_not_null())
        .filter(
            diesel::dsl::sql::<diesel::sql_types::Timestamptz>(
                "(start_setup->>'opened_at')::timestamptz",
            )
            .le(as_of - chrono::Duration::minutes(15)),
        )
        .order(tournaments::id.asc())
        .limit(limit)
        .select(tournaments::id)
        .load::<Uuid>(conn)
        .await?;
    let mut expired = Vec::new();
    for id in candidates {
        let cleared = conn
            .transaction::<_, DbError, _>(async move |tc| {
                let tournament = match Tournament::find_for_update(id, tc).await {
                    Ok(tournament) => tournament,
                    Err(DbError::NotFound { .. }) => return Ok(None),
                    Err(error) => return Err(error),
                };
                if tournament
                    .start_setup()
                    .is_none_or(|setup| setup.active_at(as_of))
                {
                    return Ok(None);
                }
                let tournament = diesel::update(tournaments::table.find(id))
                    .set((
                        tournaments::start_setup.eq(None::<serde_json::Value>),
                        tournaments::updated_at.eq(as_of),
                    ))
                    .get_result::<Tournament>(tc)
                    .await?;
                Ok(Some(tournament))
            })
            .await?;
        expired.extend(cleared);
    }
    Ok(expired)
}

pub async fn start_by_organizer(
    tournament_id: Uuid,
    organizer_id: Uuid,
    expected_entrant_ids: Vec<Uuid>,
    conn: &mut DbConn<'_>,
) -> Result<StartOutcome, DbError> {
    start_fixed_field(
        tournament_id,
        StartTrigger::Organizer(organizer_id),
        Some(expected_entrant_ids),
        None,
        conn,
    )
    .await
}

pub async fn start_scheduled(
    tournament_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<StartOutcome, DbError> {
    start_fixed_field(tournament_id, StartTrigger::Scheduled, None, None, conn).await
}

enum PreparedFixedFieldStart {
    RoundRobin(super::round_robin::PreparedRoundRobinStart),
    Swiss { rating_clock: Clock },
    Elimination(super::elimination::PreparedEliminationStart),
}

impl PreparedFixedFieldStart {
    fn new(
        configuration: &Config,
        participant_count: usize,
        bracket_order: Option<&[usize]>,
    ) -> Result<Self, DbError> {
        match &configuration.format {
            FormatConfig::RoundRobin(config) => Ok(Self::RoundRobin(
                super::round_robin::prepare_initial_start(config, participant_count)?,
            )),
            FormatConfig::Swiss(config) => Ok(Self::Swiss {
                rating_clock: config.clock,
            }),
            FormatConfig::Elimination(config) => Ok(Self::Elimination(
                super::elimination::prepare_initial_start(
                    config,
                    participant_count,
                    bracket_order,
                )?,
            )),
            FormatConfig::Arena(_) => Err(invalid_input(
                "Arena start uses its arrival-ordered format adapter",
            )),
        }
    }

    const fn rating_clock(&self) -> Clock {
        match self {
            Self::RoundRobin(prepared) => prepared.rating_clock(),
            Self::Swiss { rating_clock } => *rating_clock,
            Self::Elimination(prepared) => prepared.rating_clock(),
        }
    }

    fn initial_artifacts(self, memberships: &[TournamentUser]) -> InitialArtifacts {
        match self {
            Self::RoundRobin(prepared) => {
                super::round_robin::initial_artifacts(prepared, memberships)
            }
            Self::Swiss { .. } => InitialArtifacts::new(Vec::new(), Vec::new()),
            Self::Elimination(prepared) => {
                super::elimination::initial_artifacts(prepared, memberships)
            }
        }
    }
}

/// Why the common start transaction is being crossed.
///
/// Arena has additional late-start behavior and joins this boundary in its
/// format adapter. Fixed-field formats use both variants directly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartTrigger {
    Organizer(Uuid),
    Scheduled,
}

/// Format-owned facts accepted at the initial start boundary.
///
/// The common transaction owns persistence and release. A format adapter owns
/// only construction of its exact initial facts and logical slots.
pub(crate) struct InitialArtifacts {
    pub(crate) slots: Vec<TournamentSlotInsert>,
    pub(crate) release: Vec<SlotKey>,
    pub(crate) elimination_nodes: Vec<EliminationNodeFact>,
}

impl InitialArtifacts {
    pub(crate) fn new(slots: Vec<TournamentSlotInsert>, release: Vec<SlotKey>) -> Self {
        Self {
            slots,
            release,
            elimination_nodes: Vec::new(),
        }
    }

    pub(crate) fn with_elimination_nodes(
        mut self,
        elimination_nodes: Vec<EliminationNodeFact>,
    ) -> Self {
        self.elimination_nodes = elimination_nodes;
        self
    }
}

#[derive(Debug)]
pub struct StartOutcome {
    pub tournament: Tournament,
    pub games: Vec<Game>,
    pub removed_invitees: Vec<Uuid>,
    pub started_now: bool,
}

/// Runs the fixed-field portion of atomic start while delegating only the
/// format-owned initial decision to a synchronous adapter.
///
/// The tournament row is the fixed-field aggregate lock. Membership and active
/// account state are read only after that lock, so signup, start, and account
/// deletion have one ordering boundary without serializing unrelated games on
/// account rows.
async fn start_fixed_field(
    tournament_id: Uuid,
    trigger: StartTrigger,
    expected_entrant_ids: Option<Vec<Uuid>>,
    confirmation: Option<(Uuid, Vec<Uuid>)>,
    conn: &mut DbConn<'_>,
) -> Result<StartOutcome, DbError> {
    conn.transaction::<_, DbError, _>(async move |tc| {
        let tournament = Tournament::find_for_update(tournament_id, tc).await?;
        let started_at = Utc::now();
        let mut configuration = tournament.configuration().clone();
        authorize_start(&tournament, trigger, started_at, tc).await?;
        let setup = if let Some((setup_id, ref order)) = confirmation {
            let StartTrigger::Organizer(owner_id) = trigger else {
                return Err(invalid_input("Bracket confirmation requires its owner"));
            };
            let setup = owned_setup(&tournament, setup_id, owner_id, started_at)?;
            validate_roster(&setup.seeded_players, order)?;
            Some(setup)
        } else {
            if matches!(configuration.format, FormatConfig::Elimination(_))
                && matches!(trigger, StartTrigger::Organizer(_))
            {
                return Err(invalid_input(
                    "Elimination requires bracket review before confirmation",
                ));
            }
            None
        };
        let memberships = TournamentUser::find_by_tournament_id(tournament_id, tc).await?;
        if let Some(mut expected) = expected_entrant_ids {
            let expected_len = expected.len();
            expected.sort_unstable();
            if expected.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(DbError::InvalidAction {
                    info: String::from("Expected tournament roster contains duplicates"),
                });
            }
            let mut actual = memberships
                .iter()
                .map(|membership| membership.user_id)
                .collect::<Vec<_>>();
            actual.sort_unstable();
            if actual.len() != expected_len || actual != expected {
                return Err(DbError::InvalidAction {
                    info: String::from("Tournament roster changed before start"),
                });
            }
        }
        let configured_minimum = usize::try_from(tournament.min_seats)
            .expect("schema-constrained tournament minimum is nonnegative");
        let minimum = if matches!(&configuration.format, FormatConfig::Swiss(_)) {
            usize::try_from(MIN_SWISS_START_SEATS).expect("minimum Swiss field size fits usize")
        } else {
            configured_minimum
        };
        if memberships.len() < minimum {
            if trigger != StartTrigger::Scheduled {
                return Err(DbError::NotEnoughPlayers);
            }
            let tournament = Tournament::clear_due_scheduled_start(&tournament, tc).await?;
            return Ok(StartOutcome {
                tournament,
                games: Vec::new(),
                removed_invitees: Vec::new(),
                started_now: false,
            });
        }
        let current_user_ids = memberships
            .iter()
            .map(|membership| membership.user_id)
            .collect::<Vec<_>>();
        let bot_user_ids = User::tournament_bot_ids(&current_user_ids, tc).await?;

        let tournament = if let FormatConfig::Swiss(swiss) = &mut configuration.format {
            let unresolved = swiss.rounds.resolved_rounds().is_none();
            swiss.rounds = swiss.rounds.resolve(memberships.len()).ok_or_else(|| {
                invalid_input("Swiss round configuration cannot resolve for the active field")
            })?;
            if unresolved {
                Tournament::persist_configuration(&tournament, configuration.clone(), tc).await?
            } else {
                tournament
            }
        } else {
            tournament
        };

        let prepared = PreparedFixedFieldStart::new(&configuration, memberships.len(), None)?;
        let memberships = if let Some(setup) = &setup {
            let actual = memberships
                .iter()
                .map(|row| row.user_id)
                .collect::<Vec<_>>();
            validate_roster(&setup.seeded_players, &actual)?;
            let by_id = memberships
                .into_iter()
                .map(|row| (row.user_id, row))
                .collect::<std::collections::HashMap<_, _>>();
            setup
                .seeded_players
                .iter()
                .map(|id| by_id[id].clone())
                .collect()
        } else {
            freeze_fixed_field(prepared.rating_clock(), memberships, tc).await?
        };
        let (tournament, prepared) = if let Some((_, order)) = confirmation {
            let indices = order
                .iter()
                .map(|id| {
                    memberships
                        .iter()
                        .position(|row| row.user_id == *id)
                        .expect("validated bracket player")
                })
                .collect::<Vec<_>>();
            let tournament = diesel::update(tournaments::table.find(tournament_id))
                .set((
                    tournaments::bracket_order.eq(Some(
                        serde_json::to_value(&order).expect("serializable player order"),
                    )),
                    tournaments::start_setup.eq(None::<serde_json::Value>),
                ))
                .get_result::<Tournament>(tc)
                .await?;
            (
                tournament,
                PreparedFixedFieldStart::new(&configuration, memberships.len(), Some(&indices))?,
            )
        } else {
            (tournament, prepared)
        };
        let artifacts = prepared.initial_artifacts(&memberships);

        for (index, membership) in memberships.iter().enumerate() {
            let pairing_number =
                i32::try_from(index).expect("creation-validated tournament field size fits i32");
            TournamentUser::persist_start_fields(
                tournament_id,
                membership.user_id,
                pairing_number,
                None,
                tc,
            )
            .await?;
        }

        for fact in artifacts.elimination_nodes {
            TournamentEliminationNode::upsert(tournament_id, &fact, tc).await?;
        }
        let slot_specs = artifacts.slots;
        let inserted =
            TournamentSlot::insert_many(tournament_id, &slot_specs, started_at, tc).await?;
        let slots = inserted
            .iter()
            .map(TournamentSlot::as_slot)
            .collect::<Vec<_>>();
        let release = artifacts.release.into_iter().collect::<HashSet<_>>();
        let mut games = Vec::new();
        for slot in slots.iter().filter(|slot| release.contains(&slot.key)) {
            release_slot_into_games(
                tournament_id,
                slot,
                &mut games,
                &bot_user_ids,
                started_at,
                tc,
            )
            .await?;
        }

        let tournament = Tournament::persist_started(&tournament, started_at, tc).await?;
        let tournament = if matches!(&configuration.format, FormatConfig::Swiss(_)) {
            let mut state = load_in_progress(tournament, BotUsers::ForGameRelease, tc).await?;
            let mut progressed = progress_swiss(&mut state, started_at, tc).await?;
            games.append(&mut progressed.released_games);
            state.tournament
        } else {
            tournament
        };
        let removed_invitees = tournaments_invitations::table
            .filter(tournaments_invitations::tournament_id.eq(tournament_id))
            .select(tournaments_invitations::invitee_id)
            .load::<Uuid>(tc)
            .await?;
        diesel::delete(
            tournaments_invitations::table
                .filter(tournaments_invitations::tournament_id.eq(tournament_id)),
        )
        .execute(tc)
        .await?;

        Ok(StartOutcome {
            tournament,
            games,
            removed_invitees,
            started_now: true,
        })
    })
    .await
}

async fn authorize_start(
    tournament: &Tournament,
    trigger: StartTrigger,
    now: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    if tournament.status() != TournamentStatus::NotStarted {
        return Err(DbError::InvalidAction {
            info: String::from("Only a NotStarted tournament can start"),
        });
    }
    match (trigger, tournament.starts_at) {
        (StartTrigger::Organizer(actor), None) => {
            tournament
                .ensure_user_is_organizer_or_admin(&actor, conn)
                .await
        }
        (StartTrigger::Scheduled, Some(starts_at)) if now >= starts_at => Ok(()),
        (StartTrigger::Scheduled, Some(_)) => Err(DbError::InvalidAction {
            info: String::from("The scheduled tournament start is not due"),
        }),
        (StartTrigger::Organizer(_), Some(_)) => Err(DbError::InvalidAction {
            info: String::from("A scheduled tournament starts at its configured origin"),
        }),
        (StartTrigger::Scheduled, None) => Err(DbError::InvalidAction {
            info: String::from("An organizer-start tournament has no scheduled start"),
        }),
    }
}

async fn freeze_fixed_field(
    clock: Clock,
    memberships: Vec<TournamentUser>,
    conn: &mut DbConn<'_>,
) -> Result<Vec<TournamentUser>, DbError> {
    let speed = GameSpeed::from(clock);
    let user_ids = memberships
        .iter()
        .map(|membership| membership.user_id)
        .collect::<Vec<_>>();
    let ratings = Rating::for_uuids_at_speed(&user_ids, &speed, conn).await?;
    let ratings_by_user = ratings
        .into_iter()
        .map(|rating| (rating.user_uid, rating.rating))
        .collect::<std::collections::HashMap<_, _>>();
    let mut seeded_memberships = Vec::with_capacity(memberships.len());
    for membership in memberships {
        let rating = ratings_by_user
            .get(&membership.user_id)
            .copied()
            .ok_or_else(|| DbError::InvalidPersistedTournament {
                reason: format!(
                    "tournament entrant {} has no rating for the seeding speed",
                    membership.user_id
                ),
            })?;
        let rating =
            rating::floored(rating).ok_or_else(|| DbError::InvalidPersistedTournament {
                reason: format!(
                    "tournament entrant {} has an invalid seeding rating",
                    membership.user_id
                ),
            })?;
        seeded_memberships.push((membership, rating));
    }
    seeded_memberships.sort_unstable_by(|(left, left_rating), (right, right_rating)| {
        // Seed by the same whole-number rating shown in the prestart roster so
        // its projected schedule remains authoritative when ratings share a fraction.
        right_rating
            .cmp(left_rating)
            .then_with(|| left.user_id.cmp(&right.user_id))
    });
    let memberships = seeded_memberships
        .into_iter()
        .map(|(membership, _)| membership)
        .collect::<Vec<_>>();
    Ok(memberships)
}
