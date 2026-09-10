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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TournamentFinalOutcome {
    pub tournament_id: Uuid,
    pub standings: Snapshot,
    pub arena_ratings: Option<Vec<FrozenArenaRating>>,
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
}

#[derive(Insertable)]
#[diesel(table_name = tournament_final_outcomes)]
struct NewTournamentFinalOutcome {
    tournament_id: Uuid,
    standings: Value,
    arena_ratings: Option<Value>,
}

impl TournamentFinalOutcome {
    pub(crate) async fn insert(
        tournament_id: Uuid,
        standings: Snapshot,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let row = NewTournamentFinalOutcome {
            tournament_id,
            standings: serde_json::to_value(&standings).map_err(serialization_error)?,
            arena_ratings: None,
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
            })
            .execute(conn)
            .await?;
        let included = included_game_ids
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
        Ok(Self {
            tournament_id: row.tournament_id,
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
