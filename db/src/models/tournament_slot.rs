use crate::{db_error::DbError, models::ScheduleOffer, schema::tournament_slots, DbConn};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::{Error as JsonError, Value};
use shared_types::tournament::{Clock, Resolution, Slot, SlotKey};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct TournamentSlot {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub key: SlotKey,
    pub white: Uuid,
    pub black: Uuid,
    pub clock: Clock,
    pub resolution: Option<Resolution>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub deadline_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TournamentSlotInsert {
    pub key: SlotKey,
    pub white: Uuid,
    pub black: Uuid,
    pub clock: Clock,
    pub resolution: Option<Resolution>,
}

#[derive(Insertable)]
#[diesel(table_name = tournament_slots)]
struct NewTournamentSlot {
    tournament_id: Uuid,
    native_key: Value,
    white_id: Uuid,
    black_id: Uuid,
    clock: Value,
    resolution: Option<Value>,
    resolved_at: Option<DateTime<Utc>>,
    scheduled_at: Option<DateTime<Utc>>,
    deadline_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = tournament_slots)]
struct StoredTournamentSlot {
    id: Uuid,
    tournament_id: Uuid,
    native_key: Value,
    white_id: Uuid,
    black_id: Uuid,
    clock: Value,
    resolution: Option<Value>,
    resolved_at: Option<DateTime<Utc>>,
    scheduled_at: Option<DateTime<Utc>>,
    deadline_at: Option<DateTime<Utc>>,
}

impl TournamentSlot {
    pub async fn find(
        tournament_id: Uuid,
        tournament_slot_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let stored = tournament_slots::table
            .find(tournament_slot_id)
            .filter(tournament_slots::tournament_id.eq(tournament_id))
            .select(StoredTournamentSlot::as_select())
            .first(conn)
            .await?;
        Self::decode(stored)
    }

    pub async fn find_for_update(
        tournament_id: Uuid,
        tournament_slot_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let stored = tournament_slots::table
            .find(tournament_slot_id)
            .filter(tournament_slots::tournament_id.eq(tournament_id))
            .for_update()
            .select(StoredTournamentSlot::as_select())
            .first(conn)
            .await?;
        Self::decode(stored)
    }

    pub async fn find_by_tournament_id(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        let stored = tournament_slots::table
            .filter(tournament_slots::tournament_id.eq(tournament_id))
            .select(StoredTournamentSlot::as_select())
            .load(conn)
            .await?;
        Self::decode_many(stored)
    }

    pub async fn find_by_tournament_ids(
        tournament_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        if tournament_ids.is_empty() {
            return Ok(Vec::new());
        }
        let stored = tournament_slots::table
            .filter(tournament_slots::tournament_id.eq_any(tournament_ids))
            .select(StoredTournamentSlot::as_select())
            .load(conn)
            .await?;
        Self::decode_many(stored)
    }

    /// Locks the requested Slot rows in UUID order to keep lock acquisition stable.
    pub(crate) async fn find_by_ids_for_update(
        tournament_id: Uuid,
        tournament_slot_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        if tournament_slot_ids.is_empty() {
            return Ok(Vec::new());
        }
        let stored = tournament_slots::table
            .filter(tournament_slots::tournament_id.eq(tournament_id))
            .filter(tournament_slots::id.eq_any(tournament_slot_ids))
            .order(tournament_slots::id)
            .for_update()
            .select(StoredTournamentSlot::as_select())
            .load(conn)
            .await?;
        Self::decode_many(stored)
    }

    /// Locks Slot rows across tournaments in UUID order for account deletion.
    pub(crate) async fn find_across_tournaments_by_ids_for_update(
        tournament_slot_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        if tournament_slot_ids.is_empty() {
            return Ok(Vec::new());
        }
        let stored = tournament_slots::table
            .filter(tournament_slots::id.eq_any(tournament_slot_ids))
            .order(tournament_slots::id)
            .for_update()
            .select(StoredTournamentSlot::as_select())
            .load(conn)
            .await?;
        Self::decode_many(stored)
    }

    pub(crate) async fn insert_many(
        tournament_id: Uuid,
        slots: &[TournamentSlotInsert],
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        if slots.is_empty() {
            return Ok(Vec::new());
        }
        let rows = slots
            .iter()
            .map(|slot| Self::new_row(tournament_id, slot, effective_at))
            .collect::<Result<Vec<_>, _>>()?;
        let stored = diesel::insert_into(tournament_slots::table)
            .values(rows)
            .returning(StoredTournamentSlot::as_returning())
            .get_results::<StoredTournamentSlot>(conn)
            .await?;
        Self::decode_many(stored)
    }

    pub(crate) async fn persist_resolution(
        tournament_id: Uuid,
        slot: &Self,
        resolved_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<ScheduleOffer>, DbError> {
        Self::persist_resolution_with_cleanup_at(
            tournament_id,
            slot,
            resolved_at,
            resolved_at,
            conn,
        )
        .await
    }

    pub(crate) async fn persist_resolution_with_cleanup_at(
        tournament_id: Uuid,
        slot: &Self,
        resolved_at: DateTime<Utc>,
        schedule_cleanup_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<ScheduleOffer>, DbError> {
        let next = resolution_value(slot.resolution)?;
        let stored_resolved_at = slot.resolution.map(|_| resolved_at);
        diesel::update(tournament_slots::table.find(slot.id))
            .filter(tournament_slots::tournament_id.eq(tournament_id))
            .set((
                tournament_slots::resolution.eq(next),
                tournament_slots::resolved_at.eq(stored_resolved_at),
            ))
            .returning(tournament_slots::id)
            .get_result::<Uuid>(conn)
            .await?;
        if slot.resolution.is_some() {
            return ScheduleOffer::close_pending_for_slot(
                tournament_id,
                slot.id,
                schedule_cleanup_at,
                conn,
            )
            .await;
        }
        Ok(Vec::new())
    }

    pub async fn set_deadline_for_slots(
        tournament_id: Uuid,
        tournament_slot_ids: &[Uuid],
        deadline_at: Option<DateTime<Utc>>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        let mut tournament_slot_ids = tournament_slot_ids.to_vec();
        tournament_slot_ids.sort_unstable();
        tournament_slot_ids.dedup();
        if tournament_slot_ids.is_empty() {
            return Ok(Vec::new());
        }
        let locked =
            Self::find_by_ids_for_update(tournament_id, &tournament_slot_ids, conn).await?;
        if locked.len() != tournament_slot_ids.len()
            || locked.iter().any(|slot| slot.resolution.is_some())
        {
            return Err(DbError::InvalidAction {
                info: String::from("deadlines can only be changed for unfinished tournament Slots"),
            });
        }
        let stored = diesel::update(
            tournament_slots::table.filter(tournament_slots::id.eq_any(tournament_slot_ids)),
        )
        .set(tournament_slots::deadline_at.eq(deadline_at))
        .returning(StoredTournamentSlot::as_returning())
        .get_results(conn)
        .await?;
        Self::decode_many(stored)
    }

    pub(crate) fn as_slot(&self) -> Slot {
        Slot {
            id: self.id,
            key: self.key,
            white: self.white,
            black: self.black,
            clock: self.clock,
            resolution: self.resolution,
        }
    }

    fn decode(stored: StoredTournamentSlot) -> Result<Self, DbError> {
        let StoredTournamentSlot {
            id,
            tournament_id,
            native_key,
            white_id: white,
            black_id: black,
            clock,
            resolution,
            resolved_at,
            scheduled_at,
            deadline_at,
        } = stored;
        let invalid = |reason: String| DbError::InvalidPersistedTournament {
            reason: format!("invalid persisted tournament Slot {id}: {reason}"),
        };
        Ok(Self {
            id,
            tournament_id,
            key: serde_json::from_value(native_key)
                .map_err(|error| invalid(format!("invalid native key: {error}")))?,
            white,
            black,
            clock: serde_json::from_value(clock)
                .map_err(|error| invalid(format!("invalid clock: {error}")))?,
            resolution: resolution
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| invalid(format!("invalid resolution: {error}")))?,
            resolved_at,
            scheduled_at,
            deadline_at,
        })
    }

    fn decode_many(stored: Vec<StoredTournamentSlot>) -> Result<Vec<Self>, DbError> {
        stored.into_iter().map(Self::decode).collect()
    }

    fn new_row(
        tournament_id: Uuid,
        slot: &TournamentSlotInsert,
        effective_at: DateTime<Utc>,
    ) -> Result<NewTournamentSlot, DbError> {
        let resolved_at = slot.resolution.map(|_| effective_at);
        Ok(NewTournamentSlot {
            tournament_id,
            native_key: serde_json::to_value(slot.key).map_err(serialization_error)?,
            white_id: slot.white,
            black_id: slot.black,
            clock: serde_json::to_value(slot.clock).map_err(serialization_error)?,
            resolution: resolution_value(slot.resolution)?,
            resolved_at,
            scheduled_at: None,
            deadline_at: None,
        })
    }
}

fn resolution_value(resolution: Option<Resolution>) -> Result<Option<Value>, DbError> {
    resolution
        .map(serde_json::to_value)
        .transpose()
        .map_err(serialization_error)
}

fn serialization_error(error: JsonError) -> DbError {
    DbError::InternalError {
        reason: format!("failed to serialize tournament Slot: {error}"),
    }
}
