use crate::{
    db_error::DbError,
    models::{
        Game,
        ScheduleOffer,
        Tournament,
        TournamentEliminationNode,
        TournamentSlot,
        TournamentSwissRound,
        TournamentUser,
        User,
    },
    DbConn,
};
use hive_lib::GameStatus;
use shared_types::{
    tournament::{Config, Format, FormatConfig, GameOutcome, Resolution},
    GameId,
    TournamentStatus,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};
use uuid::Uuid;

#[cfg(test)]
pub(crate) async fn load_in_progress_for_update(
    tournament_id: Uuid,
    additional_user_ids: &[Uuid],
    bot_users: BotUsers,
    conn: &mut DbConn<'_>,
) -> Result<TournamentState, DbError> {
    let tournament = Tournament::find_for_update(tournament_id, conn).await?;
    User::ensure_active_ids(additional_user_ids, conn).await?;
    load_in_progress(tournament, bot_users, conn).await
}

pub(crate) async fn load_for_read_with_memberships(
    tournament: Tournament,
    memberships: Vec<TournamentUser>,
    conn: &mut DbConn<'_>,
) -> Result<TournamentState, DbError> {
    let tournament_id = tournament.id;
    let configuration = tournament.configuration().clone();
    let (swiss_rounds, elimination_nodes) = match &configuration.format {
        FormatConfig::RoundRobin(_) | FormatConfig::Arena(_) => (Vec::new(), Vec::new()),
        FormatConfig::Swiss(_) => (
            TournamentSwissRound::find_by_tournament_id(tournament_id, conn).await?,
            Vec::new(),
        ),
        FormatConfig::Elimination(_) => (
            Vec::new(),
            TournamentEliminationNode::find_by_tournament_id(tournament_id, conn).await?,
        ),
    };
    let slots = TournamentSlot::find_by_tournament_id(tournament_id, conn).await?;
    let games = tournament.games(conn).await?;
    Ok(TournamentState {
        tournament,
        configuration,
        memberships,
        swiss_rounds,
        elimination_nodes,
        slots,
        games,
        bot_user_ids: HashSet::new(),
        schedule_offer_updates: Vec::new(),
        fixed_field_changes: FixedFieldChanges::default(),
    })
}

pub(crate) async fn load_in_progress(
    tournament: Tournament,
    bot_users: BotUsers,
    conn: &mut DbConn<'_>,
) -> Result<TournamentState, DbError> {
    let tournament_id = tournament.id;
    if tournament.status() != TournamentStatus::InProgress {
        return Err(DbError::InvalidAction {
            info: String::from("Tournament mutation requires an in-progress tournament"),
        });
    }
    let configuration = tournament.configuration().clone();
    // The Tournament lock serializes aggregate transitions. Projection facts are
    // loaded coherently without row locks; command paths acquire the particular
    // Slot/Game rows they can mutate before calling this loader.
    let slots = TournamentSlot::find_by_tournament_id(tournament_id, conn).await?;
    let games = tournament.games(conn).await?;
    let (swiss_rounds, elimination_nodes) = match &configuration.format {
        FormatConfig::RoundRobin(_) | FormatConfig::Arena(_) => (Vec::new(), Vec::new()),
        FormatConfig::Swiss(_) => (
            TournamentSwissRound::find_by_tournament_id(tournament_id, conn).await?,
            Vec::new(),
        ),
        FormatConfig::Elimination(_) => (
            Vec::new(),
            TournamentEliminationNode::find_by_tournament_id(tournament_id, conn).await?,
        ),
    };
    let memberships = TournamentUser::find_by_tournament_id(tournament_id, conn).await?;
    let active_user_ids = memberships
        .iter()
        .filter(|membership| membership.withdrawn_at.is_none())
        .map(|membership| membership.user_id)
        .collect::<Vec<_>>();
    let bot_user_ids = match bot_users {
        BotUsers::ForGameRelease => User::tournament_bot_ids(&active_user_ids, conn).await?,
        #[cfg(test)]
        BotUsers::Skip => HashSet::new(),
    };
    Ok(TournamentState {
        tournament,
        configuration,
        memberships,
        swiss_rounds,
        elimination_nodes,
        slots,
        games,
        bot_user_ids,
        schedule_offer_updates: Vec::new(),
        fixed_field_changes: FixedFieldChanges::default(),
    })
}

#[derive(Clone, Copy)]
pub(crate) enum BotUsers {
    #[cfg(test)]
    Skip,
    ForGameRelease,
}

#[derive(Default)]
pub(crate) struct ProgressionEffects {
    pub(crate) sealed: bool,
    pub(crate) advanced: bool,
    pub(crate) released_games: Vec<Game>,
    pub(crate) finished_now: bool,
}

#[derive(Default)]
pub(crate) struct FixedFieldChanges {
    pub(crate) affected_slot_ids: Vec<Uuid>,
    pub(crate) released_game_ids: Vec<GameId>,
    pub(crate) standings_changed: bool,
    pub(crate) format_changed: bool,
    pub(crate) availability_changed: bool,
    pub(crate) catalog_changed: bool,
}

pub(crate) struct TournamentState {
    pub(crate) tournament: Tournament,
    pub(crate) configuration: Config,
    pub(crate) memberships: Vec<TournamentUser>,
    pub(crate) swiss_rounds: Vec<TournamentSwissRound>,
    pub(crate) elimination_nodes: Vec<TournamentEliminationNode>,
    pub(crate) slots: Vec<TournamentSlot>,
    pub(crate) games: Vec<Game>,
    pub(crate) bot_user_ids: HashSet<Uuid>,
    pub(crate) schedule_offer_updates: Vec<ScheduleOffer>,
    fixed_field_changes: FixedFieldChanges,
}

impl TournamentState {
    pub(crate) fn record_affected_slots(&mut self, slot_ids: impl IntoIterator<Item = Uuid>) {
        self.fixed_field_changes.affected_slot_ids.extend(slot_ids);
    }

    pub(crate) fn slot_index(&self, slot_id: Uuid) -> Result<usize, DbError> {
        self.slots
            .iter()
            .position(|slot| slot.id == slot_id)
            .ok_or_else(|| invalid_input(&format!("Tournament Slot {} is not current", slot_id)))
    }

    pub(crate) fn game(&self, game_id: Uuid) -> Result<&Game, DbError> {
        self.games
            .iter()
            .find(|game| game.id == game_id)
            .ok_or_else(|| invalid_input(&format!("Tournament game {game_id} is not current")))
    }

    pub(crate) fn slot_game(&self, slot_id: Uuid) -> Option<&Game> {
        self.games
            .iter()
            .find(|game| game.tournament_slot_id == Some(slot_id))
    }

    pub(crate) fn games_by_slot(&self) -> HashMap<Uuid, &Game> {
        self.games
            .iter()
            .filter_map(|game| game.tournament_slot_id.map(|slot_id| (slot_id, game)))
            .collect()
    }

    pub(crate) fn record_slot_resolution_change(
        &mut self,
        slot_id: Uuid,
        was_resolved: bool,
        is_resolved: bool,
    ) {
        self.fixed_field_changes.affected_slot_ids.push(slot_id);
        self.fixed_field_changes.standings_changed = true;
        self.fixed_field_changes.catalog_changed |= was_resolved != is_resolved;
        self.fixed_field_changes.availability_changed |= was_resolved != is_resolved;
        self.fixed_field_changes.format_changed |=
            !matches!(self.configuration.format(), Format::RoundRobin);
    }

    pub(crate) fn record_slot_release(&mut self, slot_id: Uuid, game: &Game) {
        self.fixed_field_changes.affected_slot_ids.push(slot_id);
        self.fixed_field_changes
            .released_game_ids
            .push(GameId(game.nanoid.clone()));
        self.fixed_field_changes.availability_changed |= matches!(
            self.configuration.format(),
            Format::Swiss | Format::DoubleSwiss
        ) && !game.finished
            && game.turn == 0
            && game.game_status == GameStatus::NotStarted.to_string();
        self.fixed_field_changes.format_changed |= matches!(
            self.configuration.format(),
            Format::SingleElimination | Format::DoubleElimination
        );
    }

    pub(crate) fn record_materialized_slots(&mut self, slots: &[TournamentSlot]) {
        self.fixed_field_changes
            .affected_slot_ids
            .extend(slots.iter().map(|slot| slot.id));
        self.fixed_field_changes.format_changed |= !slots.is_empty();
        if slots.iter().any(|slot| slot.resolution.is_some()) {
            self.fixed_field_changes.standings_changed = true;
        }
        if !slots.is_empty()
            && matches!(
                self.configuration.format(),
                Format::Swiss | Format::DoubleSwiss
            )
        {
            self.fixed_field_changes.catalog_changed = true;
        }
    }

    pub(crate) fn record_membership_change(&mut self, user_id: Uuid) {
        self.fixed_field_changes.standings_changed = true;
        self.fixed_field_changes.availability_changed = true;
        self.fixed_field_changes.format_changed |=
            !matches!(self.configuration.format(), Format::RoundRobin);
        if self.configuration.format() == Format::RoundRobin {
            self.fixed_field_changes.affected_slot_ids.extend(
                self.slots
                    .iter()
                    .filter(|slot| {
                        (slot.white == user_id || slot.black == user_id)
                            && matches!(
                                slot.resolution,
                                Some(Resolution::Result(GameOutcome::Adjudicated(_)))
                            )
                    })
                    .map(|slot| slot.id),
            );
        }
    }

    pub(crate) fn record_finish_cleanup(&mut self) {
        self.fixed_field_changes.availability_changed = true;
        self.fixed_field_changes
            .affected_slot_ids
            .extend(self.slots.iter().map(|slot| slot.id));
    }

    pub(crate) fn take_fixed_field_changes(&mut self) -> FixedFieldChanges {
        self.fixed_field_changes.affected_slot_ids.sort_unstable();
        self.fixed_field_changes.affected_slot_ids.dedup();
        self.fixed_field_changes
            .released_game_ids
            .sort_unstable_by(|left, right| left.0.cmp(&right.0));
        self.fixed_field_changes.released_game_ids.dedup();
        std::mem::take(&mut self.fixed_field_changes)
    }
}

pub(super) fn invalid_input(info: &str) -> DbError {
    DbError::InvalidInput {
        info: info.to_owned(),
        error: String::new(),
    }
}

pub(super) fn invalid_input_with(info: &str, error: impl Display) -> DbError {
    DbError::InvalidInput {
        info: info.to_owned(),
        error: error.to_string(),
    }
}

pub(super) fn invalid_persisted(reason: &str) -> DbError {
    DbError::InvalidPersistedTournament {
        reason: reason.to_owned(),
    }
}
