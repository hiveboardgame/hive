use crate::{
    db_error::DbError,
    schema::{tournament_final_arena_results, tournament_final_outcomes},
    DbConn,
};
use diesel::{prelude::*, OptionalExtension, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use serde_json::{Error as JsonError, Value};
use shared_types::tournament::standings::Snapshot;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TournamentFinishReason {
    Completed,
    PairingExhausted,
}

impl TournamentFinishReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::PairingExhausted => "pairing_exhausted",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TournamentFinalOutcome {
    pub tournament_id: Uuid,
    pub standings: Snapshot,
    pub arena_ratings: Option<Vec<FrozenArenaRating>>,
    pub finish_reason: TournamentFinishReason,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenArenaRating {
    pub user_id: Uuid,
    pub rating: i32,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(primary_key(tournament_id))]
#[diesel(table_name = tournament_final_outcomes)]
struct TournamentFinalOutcomeRow {
    tournament_id: Uuid,
    standings: Value,
    arena_ratings: Option<Value>,
    finish_reason: String,
}

#[derive(Insertable)]
#[diesel(table_name = tournament_final_outcomes)]
struct NewTournamentFinalOutcome {
    tournament_id: Uuid,
    standings: Value,
    arena_ratings: Option<Value>,
    finish_reason: String,
}

impl TournamentFinalOutcome {
    pub(crate) async fn insert(
        tournament_id: Uuid,
        standings: Snapshot,
        finish_reason: TournamentFinishReason,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let row = NewTournamentFinalOutcome {
            tournament_id,
            standings: serde_json::to_value(&standings).map_err(serialization_error)?,
            arena_ratings: None,
            finish_reason: finish_reason.as_str().to_owned(),
        };
        diesel::insert_into(tournament_final_outcomes::table)
            .values(row)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_arena(
        tournament_id: Uuid,
        standings: Snapshot,
        ratings: Vec<FrozenArenaRating>,
        included_game_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::insert_into(tournament_final_outcomes::table)
            .values(NewTournamentFinalOutcome {
                tournament_id,
                standings: serde_json::to_value(standings).map_err(serialization_error)?,
                arena_ratings: Some(serde_json::to_value(ratings).map_err(serialization_error)?),
                finish_reason: TournamentFinishReason::Completed.as_str().to_owned(),
            })
            .execute(conn)
            .await?;
        // Two bind parameters per row; keep every chunk below PostgreSQL's limit.
        // The finalization transaction owns all chunks and the snapshot atomically.
        for game_ids in included_game_ids.chunks(10_000) {
            let included = game_ids
                .iter()
                .map(|game_id| {
                    (
                        tournament_final_arena_results::tournament_id.eq(tournament_id),
                        tournament_final_arena_results::game_id.eq(*game_id),
                    )
                })
                .collect::<Vec<_>>();
            diesel::insert_into(tournament_final_arena_results::table)
                .values(included)
                .execute(conn)
                .await?;
        }
        Ok(())
    }

    pub async fn load(tournament_id: Uuid, conn: &mut DbConn<'_>) -> Result<Option<Self>, DbError> {
        tournament_final_outcomes::table
            .find(tournament_id)
            .select(TournamentFinalOutcomeRow::as_select())
            .first::<TournamentFinalOutcomeRow>(conn)
            .await
            .optional()?
            .map(Self::from_row)
            .transpose()
    }

    fn from_row(row: TournamentFinalOutcomeRow) -> Result<Self, DbError> {
        let invalid = |reason: String| DbError::InvalidPersistedTournament {
            reason: format!(
                "invalid final outcome for tournament {}: {reason}",
                row.tournament_id
            ),
        };
        let finish_reason = match row.finish_reason.as_str() {
            "completed" => TournamentFinishReason::Completed,
            "pairing_exhausted" => TournamentFinishReason::PairingExhausted,
            reason => return Err(invalid(format!("unknown finish reason: {reason}"))),
        };
        Ok(Self {
            tournament_id: row.tournament_id,
            finish_reason,
            standings: serde_json::from_value(row.standings)
                .map_err(|error| invalid(format!("invalid standings: {error}")))?,
            arena_ratings: row
                .arena_ratings
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| invalid(format!("invalid Arena ratings: {error}")))?,
        })
    }
}

fn serialization_error(error: JsonError) -> DbError {
    DbError::InternalError {
        reason: format!("failed to serialize tournament final outcome: {error}"),
    }
}
