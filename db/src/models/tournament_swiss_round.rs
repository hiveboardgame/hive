use crate::{db_error::DbError, schema::tournament_swiss_rounds, DbConn};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use serde_json::{Error as JsonError, Value};
use std::collections::{HashMap, HashSet};
use tournamint::{
    swiss::{ByeAssignment, RoundPairings},
    Pairing,
    PlayerId,
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct TournamentSwissRound {
    pub tournament_id: Uuid,
    round_id: u32,
    pairings: RoundPairings,
    accepted_ratings: HashMap<PlayerId, u32>,
    pub accepted_at: DateTime<Utc>,
}

#[derive(Identifiable, Queryable, Selectable)]
#[diesel(primary_key(tournament_id, round_id))]
#[diesel(table_name = tournament_swiss_rounds)]
struct TournamentSwissRoundRow {
    tournament_id: Uuid,
    round_id: i64,
    pairings: Value,
    accepted_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = tournament_swiss_rounds)]
struct NewTournamentSwissRound {
    tournament_id: Uuid,
    round_id: i64,
    pairings: Value,
    accepted_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AcceptedRatingSnapshot {
    player: PlayerId,
    rating: u32,
}

#[derive(Deserialize, Serialize)]
struct PersistedRoundPairings {
    games: Vec<Pairing>,
    byes: Vec<ByeAssignment>,
    accepted_ratings: Vec<AcceptedRatingSnapshot>,
}

impl TournamentSwissRound {
    pub async fn find_by_tournament_id(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        tournament_swiss_rounds::table
            .filter(tournament_swiss_rounds::tournament_id.eq(tournament_id))
            .order(tournament_swiss_rounds::round_id.asc())
            .select(TournamentSwissRoundRow::as_select())
            .load::<TournamentSwissRoundRow>(conn)
            .await?
            .into_iter()
            .map(Self::decode)
            .collect()
    }

    pub async fn find_by_tournament_ids(
        tournament_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        if tournament_ids.is_empty() {
            return Ok(Vec::new());
        }
        tournament_swiss_rounds::table
            .filter(tournament_swiss_rounds::tournament_id.eq_any(tournament_ids))
            .order((
                tournament_swiss_rounds::tournament_id,
                tournament_swiss_rounds::round_id,
            ))
            .select(TournamentSwissRoundRow::as_select())
            .load::<TournamentSwissRoundRow>(conn)
            .await?
            .into_iter()
            .map(Self::decode)
            .collect()
    }

    pub(crate) async fn insert(
        tournament_id: Uuid,
        round_id: u32,
        pairings: RoundPairings,
        rating_snapshots: HashMap<PlayerId, u32>,
        accepted_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let persisted = PersistedRoundPairings::new(
            tournament_id,
            i64::from(round_id),
            pairings,
            rating_snapshots,
        )?;
        let row = NewTournamentSwissRound {
            tournament_id,
            round_id: i64::from(round_id),
            pairings: serde_json::to_value(&persisted).map_err(serialization_error)?,
            accepted_at,
        };
        diesel::insert_into(tournament_swiss_rounds::table)
            .values(row)
            .execute(conn)
            .await?;
        let (pairings, accepted_ratings) =
            persisted.into_parts(tournament_id, i64::from(round_id))?;
        Ok(Self {
            tournament_id,
            round_id,
            pairings,
            accepted_ratings,
            accepted_at,
        })
    }

    pub fn native_round_id(&self) -> u32 {
        self.round_id
    }

    pub fn pairings(&self) -> &RoundPairings {
        &self.pairings
    }

    pub fn accepted_rating_snapshots(&self) -> &HashMap<PlayerId, u32> {
        &self.accepted_ratings
    }

    fn decode(row: TournamentSwissRoundRow) -> Result<Self, DbError> {
        let persisted = serde_json::from_value::<PersistedRoundPairings>(row.pairings)
            .map_err(|error| invalid_round(row.tournament_id, row.round_id, &error.to_string()))?;
        let round_id = u32::try_from(row.round_id).map_err(|_| {
            invalid_round(row.tournament_id, row.round_id, "round id is out of range")
        })?;
        let (pairings, accepted_ratings) = persisted.into_parts(row.tournament_id, row.round_id)?;
        Ok(Self {
            tournament_id: row.tournament_id,
            round_id,
            pairings,
            accepted_ratings,
            accepted_at: row.accepted_at,
        })
    }
}

impl PersistedRoundPairings {
    fn new(
        tournament_id: Uuid,
        round_id: i64,
        pairings: RoundPairings,
        rating_snapshots: HashMap<PlayerId, u32>,
    ) -> Result<Self, DbError> {
        let participants = round_participants(&pairings);
        if participants.len() != rating_snapshots.len()
            || participants
                .iter()
                .any(|player| !rating_snapshots.contains_key(player))
        {
            return Err(invalid_round(
                tournament_id,
                round_id,
                "accepted rating snapshots do not cover every round participant",
            ));
        }
        let mut accepted_ratings = rating_snapshots
            .into_iter()
            .map(|(player, rating)| AcceptedRatingSnapshot { player, rating })
            .collect::<Vec<_>>();
        accepted_ratings.sort_unstable_by_key(|snapshot| snapshot.player.index());
        Ok(Self {
            games: pairings.games,
            byes: pairings.byes,
            accepted_ratings,
        })
    }

    fn into_parts(
        self,
        tournament_id: Uuid,
        round_id: i64,
    ) -> Result<(RoundPairings, HashMap<PlayerId, u32>), DbError> {
        let pairings = RoundPairings {
            games: self.games,
            byes: self.byes,
        };
        let participants = round_participants(&pairings);
        let mut accepted_ratings = HashMap::with_capacity(self.accepted_ratings.len());
        for snapshot in self.accepted_ratings {
            if !participants.contains(&snapshot.player)
                || accepted_ratings
                    .insert(snapshot.player, snapshot.rating)
                    .is_some()
            {
                return Err(invalid_round(
                    tournament_id,
                    round_id,
                    "accepted rating snapshots do not match the round participants",
                ));
            }
        }
        if accepted_ratings.len() != participants.len() {
            return Err(invalid_round(
                tournament_id,
                round_id,
                "accepted rating snapshots do not cover every round participant",
            ));
        }
        Ok((pairings, accepted_ratings))
    }
}

fn round_participants(pairings: &RoundPairings) -> HashSet<PlayerId> {
    pairings
        .games
        .iter()
        .flat_map(|pairing| [pairing.white(), pairing.black()])
        .chain(pairings.byes.iter().map(|bye_| bye_.player))
        .collect()
}

fn invalid_round(tournament_id: Uuid, round_id: i64, reason: &str) -> DbError {
    DbError::InvalidPersistedTournament {
        reason: format!(
            "invalid Swiss round {}:{} facts: {reason}",
            tournament_id, round_id
        ),
    }
}

fn serialization_error(error: JsonError) -> DbError {
    DbError::InternalError {
        reason: format!("failed to serialize accepted Swiss round: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_pairings_require_every_accepted_rating_snapshot() {
        let tournament_id = Uuid::nil();
        let pairings = RoundPairings {
            games: vec![Pairing::new(PlayerId::new(0), PlayerId::new(1))],
            byes: Vec::new(),
        };
        let missing = HashMap::from([(PlayerId::new(0), 1_500)]);
        assert!(PersistedRoundPairings::new(tournament_id, 0, pairings.clone(), missing,).is_err());

        let complete = HashMap::from([(PlayerId::new(0), 1_500), (PlayerId::new(1), 1_700)]);
        let persisted = PersistedRoundPairings::new(tournament_id, 0, pairings, complete)
            .expect("persist accepted ratings");
        assert_eq!(
            persisted.accepted_ratings,
            vec![
                AcceptedRatingSnapshot {
                    player: PlayerId::new(0),
                    rating: 1_500,
                },
                AcceptedRatingSnapshot {
                    player: PlayerId::new(1),
                    rating: 1_700,
                },
            ],
        );
    }
}
