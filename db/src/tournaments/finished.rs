use crate::{db_error::DbError, DbConn};
use chrono::{DateTime, Utc};
use shared_types::tournament::Format;

use super::{
    elimination_finished_snapshot,
    persist_finished_with_outcome,
    project_elimination_facts,
    project_round_robin_facts,
    project_swiss_facts,
    round_robin::round_robin_finished_snapshot,
    state::invalid_input,
    swiss::{swiss_finished_snapshot, swiss_next_round_is_exhausted},
    TournamentState,
};

pub(crate) async fn finish_fixed_field(
    state: &TournamentState,
    finished_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    let standings = match state.configuration.format() {
        Format::RoundRobin => {
            let projection = project_round_robin_facts(state)?;
            if !projection.is_complete() {
                return Err(DbError::InvalidAction {
                    info: String::from("Round Robin is not at its Finish boundary"),
                });
            }
            round_robin_finished_snapshot(state, &projection)
        }
        Format::Swiss | Format::DoubleSwiss => {
            let projection = project_swiss_facts(state)?;
            if !projection.tournament_complete()
                && !swiss_next_round_is_exhausted(state, &projection)?
            {
                return Err(DbError::InvalidAction {
                    info: String::from("Swiss tournament is not at its Finish boundary"),
                });
            }
            swiss_finished_snapshot(state, &projection)
        }
        Format::SingleElimination | Format::DoubleElimination => {
            let projection = project_elimination_facts(state)?;
            if projection.has_active_node() || !projection.projection.complete {
                return Err(DbError::InvalidAction {
                    info: String::from("Elimination tournament is not at its Finish boundary"),
                });
            }
            elimination_finished_snapshot(state, &projection)
        }
        format => {
            return Err(invalid_input(&format!(
                "The {format} tournament adapter does not support fixed-field Finish",
            )))
        }
    };

    persist_finished_with_outcome(&state.tournament, standings, finished_at, conn).await?;
    Ok(())
}
