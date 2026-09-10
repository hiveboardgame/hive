use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::{ScheduleOfferStatus, TournamentId};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ScheduleResponse {
    pub id: Uuid,
    pub slot_id: Uuid,
    pub slot_context: String,
    pub tournament_name: String,
    pub tournament_id: TournamentId,
    pub proposer_id: Uuid,
    pub proposer_username: String,
    pub opponent_id: Uuid,
    pub opponent_username: String,
    pub candidate_times: Vec<DateTime<Utc>>,
    pub status: ScheduleOfferStatus,
    pub selected_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub notified: bool,
}

impl ScheduleResponse {
    pub fn is_pending(&self) -> bool {
        self.status == ScheduleOfferStatus::Pending
    }

    pub fn has_future_candidate(&self, now: DateTime<Utc>) -> bool {
        self.candidate_times
            .iter()
            .any(|candidate| *candidate > now)
    }
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use anyhow::{anyhow, Result};
use db_lib::{
    models::{ScheduleOffer, Tournament, User},
    schema::{tournament_slots, tournaments_users},
    tournaments::{build_round_robin_definition, build_single_elimination_bracket, build_double_elimination_bracket},
    DbConn,
};
use diesel::{dsl::count_star, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use std::collections::{HashMap, HashSet};
use shared_types::tournament::{elimination::{Stage, Topology as EliminationTopology}, Format, FormatConfig, SlotKey, SwissLeg};
use serde_json::Value;

fn schedule_slot_contexts(tournament: &Tournament, participant_count: usize, keys: Vec<SlotKey>) -> Result<HashMap<SlotKey, String>> {
    // These definitions are the same persisted configuration projections used by the tournament view.
    // TODO: i18n once copy is approved.
    let keys = keys.into_iter().collect::<HashSet<_>>();
    let mut contexts = HashMap::new();
    match &tournament.configuration().format {
        FormatConfig::RoundRobin(config) => {
            let definition = build_round_robin_definition(config, participant_count)?;
            for round in definition.rounds {
                for game in round.games {
                    let key = SlotKey::RoundRobin { slot: game.id };
                    if keys.contains(&key) {
                        let base = format!("Round {} · Board {}", game.round_index + 1, game.board_index + 1);
                        contexts.insert(key, if config.repeats.get() > 1 {
                            format!("{base} · Game {} of {}", game.pass_index + 1, config.repeats.get())
                        } else { base });
                    }
                }
            }
        }
        FormatConfig::Swiss(config) => {
            for key in keys {
                if let SlotKey::Swiss { slot } = key {
                    let base = format!("Round {} · Board {}", slot.round_index + 1, slot.pairing_index + 1);
                    contexts.insert(key, if FormatConfig::Swiss(config.clone()).kind() == Format::DoubleSwiss {
                        format!("{base} · Game {} of 2", if slot.leg == SwissLeg::First { 1 } else { 2 })
                    } else { base });
                }
            }
        }
        FormatConfig::Elimination(config) => {
            let definition = match config.topology {
                EliminationTopology::Single { .. } => build_single_elimination_bracket(config, participant_count)?,
                EliminationTopology::Double => build_double_elimination_bracket(config, participant_count)?,
            };
            let nodes = definition.nodes().iter().map(|node| (node.id, node)).collect::<HashMap<_, _>>();
            let mut stage_counts = HashMap::new();
            for node in definition.nodes() { *stage_counts.entry(node.stage).or_insert(0_usize) += 1; }
            for key in keys {
                if let SlotKey::Elimination { node, slot } = key {
                    let descriptor = nodes.get(&node)
                        .ok_or_else(|| anyhow!("Schedule elimination node is outside its definition"))?;
                    let stage = match descriptor.stage {
                        Stage::SingleRound { round_index } => format!("Round {}", round_index + 1),
                        Stage::SingleFinal => String::from("Final"),
                        Stage::Bronze => String::from("Third place"),
                        Stage::WinnersRound { round_index } => format!("Winners round {}", round_index + 1),
                        Stage::LosersMinor { .. } | Stage::LosersMajor { .. } => format!("Losers round {}", descriptor.stage.lower_round_ordinal().unwrap_or_default() + 1),
                        Stage::GrandFinal => String::from("Grand Final"),
                        Stage::Reset => String::from("Grand Final · Deciding series"),
                    };
                    let match_label = if stage_counts[&descriptor.stage] > 1 {
                        format!(" · Match {}", descriptor.stage_ordinal + 1)
                    } else { String::new() };
                    contexts.insert(key, format!("{stage}{match_label} · Game {}", slot.value() + 1));
                }
            }
        }
        FormatConfig::Arena(_) => {}
    }
    Ok(contexts)
}

impl ScheduleResponse {
    pub async fn from_model(offer: ScheduleOffer, conn: &mut DbConn<'_>) -> Result<Self> {
        Self::from_models_batch(vec![offer], conn).await?.pop()
            .ok_or_else(|| anyhow!("Schedule response data was not assembled"))
    }

    pub async fn from_models_batch(
        offers: Vec<ScheduleOffer>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>> {
        if offers.is_empty() {
            return Ok(Vec::new());
        }

        let mut user_ids = HashSet::new();
        let mut tournament_ids = HashSet::new();
        let mut slot_ids = HashSet::new();
        for offer in &offers {
            tournament_ids.insert(offer.tournament_id);
            slot_ids.insert(offer.tournament_slot_id);
            user_ids.insert(offer.proposer_id);
        }

        let tournament_ids = tournament_ids.into_iter().collect::<Vec<_>>();
        let slot_ids = slot_ids.into_iter().collect::<Vec<_>>();
        let slot_entrants = tournament_slots::table
            .filter(tournament_slots::id.eq_any(&slot_ids))
            .select((
                tournament_slots::tournament_id,
                tournament_slots::id,
                tournament_slots::white_id,
                tournament_slots::black_id,
                tournament_slots::native_key,
            ))
            .load::<(Uuid, Uuid, Uuid, Uuid, Value)>(conn)
            .await?
            .into_iter()
            .map(|(tournament_id, slot_id, white_id, black_id, key)| {
                Ok(((tournament_id, slot_id), (white_id, black_id, serde_json::from_value::<SlotKey>(key)?)))
            })
            .collect::<Result<HashMap<_, _>, serde_json::Error>>()?;
        let opponent_ids = offers
            .iter()
            .map(|offer| {
                let (white_id, black_id, _) = slot_entrants
                    .get(&(offer.tournament_id, offer.tournament_slot_id))
                    .ok_or_else(|| anyhow!("Schedule slot response data was not loaded"))?;
                Ok((
                    offer.id,
                    ScheduleOffer::opponent_id(offer.proposer_id, *white_id, *black_id),
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        user_ids.extend(opponent_ids.values().copied());
        let user_ids = user_ids.into_iter().collect::<Vec<_>>();
        let tournaments = Tournament::find_by_uuids(&tournament_ids, conn).await?;
        let users = User::find_by_uuids(&user_ids, conn).await?;

        let participant_counts = tournaments_users::table
            .filter(tournaments_users::tournament_id.eq_any(&tournament_ids))
            .group_by(tournaments_users::tournament_id)
            .select((tournaments_users::tournament_id, count_star()))
            .load::<(Uuid, i64)>(conn).await?.into_iter().collect::<HashMap<_, _>>();
        let contexts = tournaments.iter().map(|tournament| {
            let keys = slot_entrants.iter().filter(|((id, _), _)| *id == tournament.id)
                .map(|(_, (_, _, key))| *key)
                .collect::<Vec<_>>();
            Ok((tournament.id, schedule_slot_contexts(tournament, participant_counts.get(&tournament.id).copied().unwrap_or_default() as usize, keys)?))
        }).collect::<Result<HashMap<_, _>>>()?;
        let tournaments = tournaments
            .into_iter()
            .map(|tournament| (tournament.id, tournament))
            .collect::<HashMap<_, _>>();
        let users = users
            .into_iter()
            .map(|user| (user.id, user))
            .collect::<HashMap<_, _>>();

        offers
            .into_iter()
            .map(|offer| {
                let opponent_id = *opponent_ids
                    .get(&offer.id)
                    .ok_or_else(|| anyhow!("Schedule opponent response data was not assembled"))?;
                let tournament = tournaments
                    .get(&offer.tournament_id)
                    .ok_or_else(|| anyhow!("Schedule tournament response data was not loaded"))?;
                let proposer = users
                    .get(&offer.proposer_id)
                    .ok_or_else(|| anyhow!("Schedule proposer response data was not loaded"))?;
                let opponent = users
                    .get(&opponent_id)
                    .ok_or_else(|| anyhow!("Schedule opponent user response data was not loaded"))?;
                let key = slot_entrants.get(&(offer.tournament_id, offer.tournament_slot_id))
                    .ok_or_else(|| anyhow!("Schedule slot response data was not loaded"))?.2;
                let context = contexts.get(&offer.tournament_id).and_then(|contexts| contexts.get(&key))
                    .cloned().ok_or_else(|| anyhow!("Schedule slot context was not assembled"))?;
                Self::from_parts(
                    offer,
                    tournament,
                    proposer,
                    opponent_id,
                    opponent,
                    context,
                )
            })
            .collect()
    }

    fn from_parts(
        offer: ScheduleOffer,
        tournament: &Tournament,
        proposer: &User,
        opponent_id: Uuid,
        opponent: &User,
        slot_context: String,
    ) -> Result<Self> {
        Ok(Self {
            id: offer.id,
            slot_id: offer.tournament_slot_id,
            slot_context,
            tournament_name: tournament.name.clone(),
            tournament_id: TournamentId(tournament.nanoid.clone()),
            proposer_id: offer.proposer_id,
            proposer_username: proposer.username.clone(),
            opponent_id,
            opponent_username: opponent.username.clone(),
            candidate_times: offer.candidates()?,
            status: offer.status()?,
            selected_time: offer.selected_time,
            created_at: offer.created_at,
            resolved_at: offer.resolved_at,
            resolved_by: offer.resolved_by,
            notified: offer.notified,
        })
    }
}
}}
