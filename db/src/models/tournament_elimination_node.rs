use crate::{db_error::DbError, schema::tournament_elimination_nodes, DbConn};
use diesel::{prelude::*, upsert::excluded, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde_json::{Error as JsonError, Value};
use std::collections::HashMap;
use tournamint::elimination::{EliminationNodeFact, EliminationNodeFactState};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EliminationProgress {
    pub decided_nodes: usize,
    pub total_nodes: usize,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(primary_key(tournament_id, node_id))]
#[diesel(table_name = tournament_elimination_nodes)]
pub struct TournamentEliminationNode {
    pub tournament_id: Uuid,
    pub node_id: i64,
    fact: Value,
}

#[derive(Insertable)]
#[diesel(table_name = tournament_elimination_nodes)]
struct NewTournamentEliminationNode {
    tournament_id: Uuid,
    node_id: i64,
    fact: Value,
}

impl TournamentEliminationNode {
    pub async fn find_by_tournament_id(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(tournament_elimination_nodes::table
            .filter(tournament_elimination_nodes::tournament_id.eq(tournament_id))
            .order(tournament_elimination_nodes::node_id.asc())
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    pub async fn progress_for_tournaments(
        tournament_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<HashMap<Uuid, EliminationProgress>, DbError> {
        let rows = tournament_elimination_nodes::table
            .filter(tournament_elimination_nodes::tournament_id.eq_any(tournament_ids))
            .order((
                tournament_elimination_nodes::tournament_id,
                tournament_elimination_nodes::node_id,
            ))
            .select(Self::as_select())
            .load::<Self>(conn)
            .await?;
        let mut progress = HashMap::<Uuid, EliminationProgress>::new();
        for row in rows {
            let fact = row.fact()?;
            let entry = progress.entry(row.tournament_id).or_default();
            entry.total_nodes += 1;
            if matches!(
                fact.state,
                EliminationNodeFactState::Resolved { .. } | EliminationNodeFactState::Skipped
            ) {
                entry.decided_nodes += 1;
            }
        }
        Ok(progress)
    }

    pub(crate) async fn upsert(
        tournament_id: Uuid,
        fact: &EliminationNodeFact,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let row = NewTournamentEliminationNode {
            tournament_id,
            node_id: fact.node.value() as i64,
            fact: serde_json::to_value(fact).map_err(serialization_error)?,
        };
        Ok(diesel::insert_into(tournament_elimination_nodes::table)
            .values(row)
            .on_conflict((
                tournament_elimination_nodes::tournament_id,
                tournament_elimination_nodes::node_id,
            ))
            .do_update()
            .set(
                tournament_elimination_nodes::fact.eq(excluded(tournament_elimination_nodes::fact)),
            )
            .returning(Self::as_returning())
            .get_result(conn)
            .await?)
    }

    pub fn fact(&self) -> Result<EliminationNodeFact, DbError> {
        serde_json::from_value(self.fact.clone()).map_err(|error| {
            DbError::InvalidPersistedTournament {
                reason: format!(
                    "invalid elimination node {}:{} fact: {error}",
                    self.tournament_id, self.node_id
                ),
            }
        })
    }
}

fn serialization_error(error: JsonError) -> DbError {
    DbError::InternalError {
        reason: format!("failed to serialize elimination node fact: {error}"),
    }
}
