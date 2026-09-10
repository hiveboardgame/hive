use crate::{
    db_error::DbError,
    models::{tournament::Tournament, user::User},
    schema::tournaments_users::{self, dsl::tournaments_users as tournament_user_table},
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, Identifiable, Insertable, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use shared_types::tournament::arena::PairingIntent;
use std::fmt::Display;
use uuid::Uuid;

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Tournament))]
#[diesel(table_name = tournaments_users)]
#[diesel(primary_key(tournament_id, user_id))]
pub struct TournamentUser {
    pub tournament_id: Uuid,
    pub user_id: Uuid,
    pub accepted_at: DateTime<Utc>,
    pub pairing_number: Option<i32>,
    pub arena_rating: Option<i32>,
    pub arena_pairing_intent: Option<String>,
    pub arena_waiting_since: Option<DateTime<Utc>>,
    pub withdrawn_at: Option<DateTime<Utc>>,
}

impl TournamentUser {
    pub fn accepted_at(tournament_id: Uuid, user_id: Uuid, accepted_at: DateTime<Utc>) -> Self {
        Self {
            tournament_id,
            user_id,
            accepted_at,
            pairing_number: None,
            arena_rating: None,
            arena_pairing_intent: None,
            arena_waiting_since: None,
            withdrawn_at: None,
        }
    }

    pub fn accepted_for_arena(
        tournament_id: Uuid,
        user_id: Uuid,
        accepted_at: DateTime<Utc>,
    ) -> Self {
        let mut participant = Self::accepted_at(tournament_id, user_id, accepted_at);
        participant.arena_pairing_intent = Some(pairing_intent_to_sql(PairingIntent::Enabled));
        participant
    }

    pub fn pairing_intent(&self) -> Result<Option<PairingIntent>, DbError> {
        self.arena_pairing_intent
            .as_deref()
            .map(pairing_intent_from_sql)
            .transpose()
    }

    pub(crate) async fn next_arena_pairing_number(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<i32, DbError> {
        let maximum: Option<i32> = tournaments_users::table
            .filter(tournaments_users::tournament_id.eq(tournament_id))
            .select(diesel::dsl::max(tournaments_users::pairing_number))
            .first(conn)
            .await?;
        maximum
            .map_or(Some(0), |number| number.checked_add(1))
            .ok_or_else(|| DbError::InvalidPersistedTournament {
                reason: String::from("Arena pairing number exceeds the storage domain"),
            })
    }

    pub async fn find_by_tournament_id(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(tournament_user_table
            .filter(tournaments_users::tournament_id.eq(tournament_id))
            .order((
                tournaments_users::pairing_number.asc().nulls_last(),
                tournaments_users::accepted_at.asc(),
                tournaments_users::user_id.asc(),
            ))
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    /// Locks all tournament memberships in the same user-ID order used by
    /// targeted multi-player Arena settlement.
    pub async fn find_by_tournament_id_for_update(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(tournament_user_table
            .filter(tournaments_users::tournament_id.eq(tournament_id))
            .order(tournaments_users::user_id.asc())
            .for_update()
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    pub(crate) async fn find_for_update(
        tournament_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<Self>, DbError> {
        Ok(tournament_user_table
            .find((tournament_id, user_id))
            .for_update()
            .select(Self::as_select())
            .first(conn)
            .await
            .optional()?)
    }

    /// Locks the requested membership rows in deterministic user-ID order.
    pub(crate) async fn find_by_user_ids_for_update(
        tournament_id: Uuid,
        user_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(tournaments_users::table
            .filter(tournaments_users::tournament_id.eq(tournament_id))
            .filter(tournaments_users::user_id.eq_any(user_ids))
            .order(tournaments_users::user_id.asc())
            .for_update()
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    pub async fn contains(
        tournament_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        Ok(diesel::select(diesel::dsl::exists(
            tournament_user_table.find((tournament_id, user_id)),
        ))
        .get_result(conn)
        .await?)
    }

    pub(crate) async fn insert(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.insert_into(tournament_user_table)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn persist_start_fields(
        tournament_id: Uuid,
        user_id: Uuid,
        pairing_number: i32,
        arena_rating: Option<i32>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let pairing_intent = arena_rating.map(|_| pairing_intent_to_sql(PairingIntent::Enabled));
        Ok(
            diesel::update(tournament_user_table.find((tournament_id, user_id)))
                .set((
                    tournaments_users::pairing_number.eq(Some(pairing_number)),
                    tournaments_users::arena_rating.eq(arena_rating),
                    tournaments_users::arena_pairing_intent.eq(pairing_intent),
                ))
                .get_result(conn)
                .await?,
        )
    }

    pub(crate) async fn persist_withdrawal(
        tournament_id: Uuid,
        user_id: Uuid,
        withdrawn_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        Ok(
            diesel::update(tournament_user_table.find((tournament_id, user_id)))
                .set((
                    tournaments_users::withdrawn_at.eq(Some(withdrawn_at)),
                    tournaments_users::arena_waiting_since.eq(None::<DateTime<Utc>>),
                ))
                .get_result(conn)
                .await?,
        )
    }

    pub(crate) async fn insert_live_arena(
        tournament_id: Uuid,
        user_id: Uuid,
        accepted_at: DateTime<Utc>,
        pairing_number: i32,
        arena_rating: i32,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let membership = Self {
            tournament_id,
            user_id,
            accepted_at,
            pairing_number: Some(pairing_number),
            arena_rating: Some(arena_rating),
            arena_pairing_intent: Some(pairing_intent_to_sql(PairingIntent::Enabled)),
            arena_waiting_since: Some(accepted_at),
            withdrawn_at: None,
        };
        Ok(membership
            .insert_into(tournament_user_table)
            .get_result(conn)
            .await?)
    }

    pub(crate) async fn persist_arena_pairing_state(
        tournament_id: Uuid,
        user_id: Uuid,
        next: PairingIntent,
        waiting_since: Option<DateTime<Utc>>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let waiting_since = match next {
            PairingIntent::Enabled => waiting_since,
            PairingIntent::Paused => None,
        };
        Ok(
            diesel::update(tournament_user_table.find((tournament_id, user_id)))
                .set((
                    tournaments_users::arena_pairing_intent.eq(pairing_intent_to_sql(next)),
                    tournaments_users::arena_waiting_since.eq(waiting_since),
                ))
                .get_result(conn)
                .await?,
        )
    }

    pub(crate) async fn persist_arena_completion_state(
        tournament_id: Uuid,
        user_id: Uuid,
        arena_rating: i32,
        pairing_intent: Option<PairingIntent>,
        waiting_since: Option<DateTime<Utc>>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let waiting_since = match pairing_intent {
            Some(PairingIntent::Enabled) => waiting_since,
            Some(PairingIntent::Paused) | None => None,
        };
        Ok(
            diesel::update(tournament_user_table.find((tournament_id, user_id)))
                .set((
                    tournaments_users::arena_rating.eq(Some(arena_rating)),
                    tournaments_users::arena_pairing_intent
                        .eq(pairing_intent.map(pairing_intent_to_sql)),
                    tournaments_users::arena_waiting_since.eq(waiting_since),
                ))
                .get_result(conn)
                .await?,
        )
    }

    pub(crate) async fn delete(
        tournament_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::delete(tournaments_users::table.find((tournament_id, user_id)))
            .execute(conn)
            .await?;
        Ok(())
    }
}

fn pairing_intent_to_sql(intent: PairingIntent) -> String {
    match intent {
        PairingIntent::Enabled => String::from("enabled"),
        PairingIntent::Paused => String::from("paused"),
    }
}

fn pairing_intent_from_sql(value: &str) -> Result<PairingIntent, DbError> {
    match value {
        "enabled" => Ok(PairingIntent::Enabled),
        "paused" => Ok(PairingIntent::Paused),
        value => Err(invalid_persisted_participant(
            "arena_pairing_intent",
            format_args!("unknown value {value:?}"),
        )),
    }
}

fn invalid_persisted_participant(field: &'static str, error: impl Display) -> DbError {
    DbError::InvalidPersistedTournament {
        reason: format!("invalid tournaments_users.{field}: {error}"),
    }
}
