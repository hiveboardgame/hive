use super::model::*;
use anyhow::{ensure, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared_types::{
    tournament::{Clock, Resolution, SlotKey},
    TournamentGameResult,
    TournamentStatus,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    str::FromStr,
};
use tournamint::{swiss::RoundPairings, Pairing, PlayerId};
use uuid::Uuid;

pub const BUNDLE_ORDERING: &str =
    "tournament,organizer,membership,invitation,swiss_round,slot,game,schedule_offer,final_outcome";
pub const SKIPPED_SCHEDULES_WITHOUT_FINAL_MEANING: &str = "schedule_without_final_schema_meaning";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalOrganizer {
    pub tournament_id: Uuid,
    pub organizer_id: Uuid,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInvitation {
    pub tournament_id: Uuid,
    pub invitee_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub declined_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalGame {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub key: SlotKey,
    pub tournament_game_result: TournamentGameResult,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalScheduleOffer {
    pub tournament_id: Uuid,
    pub key: SlotKey,
    pub proposer_id: Uuid,
    pub candidate_times: Vec<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<&LegacyOrganizerRow> for CanonicalOrganizer {
    fn from(x: &LegacyOrganizerRow) -> Self {
        Self {
            tournament_id: x.tournament_id,
            organizer_id: x.organizer_id,
        }
    }
}
impl From<&LegacyInvitationRow> for CanonicalInvitation {
    fn from(x: &LegacyInvitationRow) -> Self {
        Self {
            tournament_id: x.tournament_id,
            invitee_id: x.invitee_id,
            created_at: x.created_at,
            declined_at: x.declined_at,
        }
    }
}
impl CanonicalGame {
    fn from_legacy(x: &LegacyGameRow, key: SlotKey) -> Result<Self> {
        Ok(Self {
            id: x.id,
            tournament_id: x.tournament_id.expect("filtered tournament game"),
            key,
            tournament_game_result: x.tournament_game_result.parse().with_context(|| {
                format!(
                    "game {} has an unsupported source result {:?}",
                    x.id, x.tournament_game_result
                )
            })?,
            finished_at: x.finished.then_some(x.updated_at),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "record_type",
    content = "contents",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum BundleRecord {
    Tournament {
        tournament: Box<CanonicalTournament>,
    },
    Organizer {
        organizer: CanonicalOrganizer,
    },
    Membership {
        membership: CanonicalMembership,
    },
    Invitation {
        invitation: CanonicalInvitation,
    },
    SwissRound {
        round: CanonicalSwissRound,
    },
    Slot {
        slot: CanonicalSlot,
    },
    Game {
        game: CanonicalGame,
    },
    ScheduleOffer {
        offer: CanonicalScheduleOffer,
    },
    FinalOutcome {
        outcome: CanonicalFinalOutcome,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleManifest {
    pub source_identifier: String,
    pub exported_at: DateTime<Utc>,
    pub ordering: String,
    pub record_counts: BTreeMap<String, u64>,
    pub skipped_record_counts: BTreeMap<String, u64>,
    pub jsonl_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedBundle {
    pub jsonl: Vec<u8>,
    pub manifest: BundleManifest,
}

impl EncodedBundle {
    pub(crate) fn build(source: &LegacySource, audit: &AuditOutcome) -> Result<Self> {
        ensure!(
            !audit.has_hard_failures(),
            "a bundle cannot be built from an audit with hard failures"
        );
        let exported_at = Utc::now();
        let (records, retained_schedule_rows) = records(source, audit, exported_at)?;
        let mut jsonl = Vec::new();
        for record in &records {
            serde_json::to_writer(&mut jsonl, record)?;
            jsonl.push(b'\n');
        }
        let manifest = BundleManifest {
            source_identifier: source.metadata.database_name.clone(),
            exported_at,
            ordering: BUNDLE_ORDERING.into(),
            record_counts: record_counts(&records),
            skipped_record_counts: skipped_record_counts(source, retained_schedule_rows)?,
            jsonl_sha256: sha256_hex(&jsonl),
        };
        validate(&jsonl, &manifest)?;
        Ok(Self { jsonl, manifest })
    }
}

pub fn validate(jsonl: &[u8], m: &BundleManifest) -> Result<Vec<BundleRecord>> {
    ensure!(
        m.ordering == BUNDLE_ORDERING,
        "unknown bundle ordering contract"
    );
    ensure!(
        m.jsonl_sha256 == sha256_hex(jsonl),
        "bundle checksum does not match its manifest"
    );
    ensure!(
        jsonl.is_empty() || jsonl.last() == Some(&b'\n'),
        "JSONL must end with a newline"
    );
    let mut out = Vec::new();
    for (i, line) in jsonl
        .strip_suffix(b"\n")
        .unwrap_or(jsonl)
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .enumerate()
    {
        let row: BundleRecord = serde_json::from_slice(line)
            .with_context(|| format!("invalid JSONL record at line {}", i + 1))?;
        out.push(row)
    }
    ensure!(
        record_counts(&out) == m.record_counts,
        "bundle record counts do not match its manifest"
    );
    validate_swiss_rating_snapshots(&out)?;
    ensure!(
        m.skipped_record_counts
            .keys()
            .map(String::as_str)
            .eq([SKIPPED_SCHEDULES_WITHOUT_FINAL_MEANING,]),
        "bundle skipped-record accounting has an unknown shape"
    );
    sort_records(&mut out);
    Ok(out)
}

fn validate_swiss_rating_snapshots(records: &[BundleRecord]) -> Result<()> {
    for round in records.iter().filter_map(|record| match record {
        BundleRecord::SwissRound { round } => Some(round),
        _ => None,
    }) {
        let participants = round
            .pairings
            .games
            .iter()
            .flat_map(|pairing| [pairing.white(), pairing.black()])
            .chain(round.pairings.byes.iter().map(|bye_| bye_.player))
            .collect::<BTreeSet<_>>();
        let snapshots = round
            .accepted_ratings
            .iter()
            .map(|snapshot| snapshot.player)
            .collect::<BTreeSet<_>>();
        ensure!(
            round.accepted_ratings.len() == participants.len() && snapshots == participants,
            "Swiss round {}:{} accepted ratings do not cover every participant exactly once",
            round.tournament_id,
            round.round_id,
        );
    }
    Ok(())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn records(
    source: &LegacySource,
    audit: &AuditOutcome,
    cutover_at: DateTime<Utc>,
) -> Result<(Vec<BundleRecord>, usize)> {
    let games_by_id = source
        .games
        .iter()
        .rev()
        .map(|game| (game.id, game))
        .collect::<HashMap<_, _>>();
    let mut out = Vec::new();
    let mut created_at_by_id = HashMap::new();
    let plans: HashMap<_, _> = audit.plans.iter().map(|p| (p.tournament.id, p)).collect();
    let schedule_projection = project_schedules(source, audit, cutover_at)?;
    for row in &source.tournaments {
        let p = plans[&row.id];
        let lifecycle = if p.final_outcome.is_some() {
            TournamentStatus::Finished
        } else {
            match LegacyTournamentStatus::from_str(&row.status)? {
                LegacyTournamentStatus::NotStarted => TournamentStatus::NotStarted,
                LegacyTournamentStatus::InProgress => TournamentStatus::InProgress,
                LegacyTournamentStatus::Finished => TournamentStatus::Finished,
            }
        };
        ensure!(
            p.final_outcome.is_some() == (lifecycle == TournamentStatus::Finished),
            "tournament {} final outcome does not match its canonical lifecycle",
            row.id
        );
        let created_at = reconstructed_created_at(source, row);
        // Legacy manual tournaments can still say InProgress after their last
        // result. Their finish time must include that result, not just startup.
        let finished_at = (lifecycle == TournamentStatus::Finished).then(|| {
            p.slots
                .iter()
                .filter_map(|slot| match slot.status {
                    SlotPlanStatus::Sealed { game_id, .. } => {
                        games_by_id.get(&game_id).map(|game| game.updated_at)
                    }
                    _ => None,
                })
                .chain(row.started_at)
                .fold(row.updated_at, |latest, time| latest.max(time))
        });
        created_at_by_id.insert(row.id, created_at);
        out.push(BundleRecord::Tournament {
            tournament: Box::new(CanonicalTournament {
                // The final-schema row keeps the legacy tournament identity so
                // every source relationship remains a direct bijection.
                id: row.id,
                nanoid: row.nanoid.clone(),
                name: row.name.clone(),
                description: Some(row.description.clone()),
                seats: row.seats,
                min_seats: row.min_seats,
                invite_only: row.invite_only,
                band_upper: row.band_upper,
                band_lower: row.band_lower,
                starts_at: row.starts_at,
                started_at: row.started_at,
                created_at,
                updated_at: row.updated_at,
                series: row.series,
                configuration: p.configuration.clone(),
                lifecycle,
                finished_at,
            }),
        });
    }
    for x in &source.organizers {
        out.push(BundleRecord::Organizer {
            organizer: x.into(),
        })
    }
    for p in &audit.plans {
        let pairing: HashMap<_, _> = p
            .roster
            .as_ref()
            .map(|r| {
                r.participants
                    .iter()
                    .map(|x| (x.user_id, x.pairing_number as i32 - 1))
                    .collect()
            })
            .unwrap_or_default();
        for x in source
            .memberships
            .iter()
            .filter(|x| x.tournament_id == p.tournament.id)
        {
            out.push(BundleRecord::Membership {
                membership: CanonicalMembership {
                    tournament_id: x.tournament_id,
                    user_id: x.user_id,
                    accepted_at: created_at_by_id[&p.tournament.id],
                    pairing_number: pairing.get(&x.user_id).copied(),
                },
            })
        }
    }
    for x in &source.invitations {
        out.push(BundleRecord::Invitation {
            invitation: x.into(),
        })
    }
    for p in &audit.plans {
        let ids: HashMap<_, _> = p
            .roster
            .as_ref()
            .map(|r| {
                r.participants
                    .iter()
                    .map(|x| (x.user_id, PlayerId::new((x.pairing_number - 1) as usize)))
                    .collect()
            })
            .unwrap_or_default();
        for r in &p.accepted_swiss_rounds {
            let accepted_at = p
                .slots
                .iter()
                .filter(|slot| {
                    matches!(
                        slot.key,
                        SlotKey::Swiss { slot: game }
                            if game.round_index == r.round_index
                    )
                })
                .filter_map(|slot| match slot.status {
                    SlotPlanStatus::Released { game_id }
                    | SlotPlanStatus::Sealed { game_id, .. } => Some(game_id),
                    SlotPlanStatus::Planned => None,
                })
                .filter_map(|game_id| games_by_id.get(&game_id).map(|game| game.created_at))
                .min()
                .unwrap_or_else(|| p.tournament.started_at.unwrap_or(p.tournament.created_at));
            out.push(BundleRecord::SwissRound {
                round: CanonicalSwissRound {
                    tournament_id: p.tournament.id,
                    round_id: i64::from(r.round_index),
                    pairings: RoundPairings {
                        games: r
                            .pairings
                            .iter()
                            .map(|g| Pairing::new(ids[&g.white_id], ids[&g.black_id]))
                            .collect(),
                        byes: Vec::new(),
                    },
                    accepted_ratings: {
                        let mut snapshots = r
                            .accepted_ratings
                            .iter()
                            .map(|snapshot| CanonicalAcceptedRating {
                                player: ids[&snapshot.user_id],
                                rating: snapshot.rating,
                            })
                            .collect::<Vec<_>>();
                        snapshots.sort_unstable_by_key(|snapshot| snapshot.player.index());
                        snapshots
                    },
                    accepted_at,
                },
            })
        }
        for s in &p.slots {
            let (resolution, resolved_at) = match s.status {
                SlotPlanStatus::Sealed { game_id, outcome } => {
                    let resolved_at = games_by_id
                        .get(&game_id)
                        .with_context(|| {
                            format!(
                                "sealed canonical slot {}:{:?} references missing game {game_id}",
                                p.tournament.id, s.key
                            )
                        })?
                        .updated_at;
                    (Some(Resolution::Result(outcome)), Some(resolved_at))
                }
                _ => (None, None),
            };
            out.push(BundleRecord::Slot {
                slot: CanonicalSlot {
                    tournament_id: p.tournament.id,
                    key: s.key,
                    white_id: s.white_id,
                    black_id: s.black_id,
                    clock: s.clock,
                    resolution,
                    resolved_at,
                    scheduled_at: schedule_projection
                        .scheduled_at
                        .get(&(p.tournament.id, s.key))
                        .copied(),
                    deadline_at: None,
                },
            })
        }
        let slots_by_game =
            p.slots
                .iter()
                .rev()
                .filter_map(|slot| match slot.status {
                    SlotPlanStatus::Released { game_id }
                    | SlotPlanStatus::Sealed { game_id, .. } => Some((game_id, slot)),
                    SlotPlanStatus::Planned => None,
                })
                .collect::<HashMap<_, _>>();
        for g in source
            .games
            .iter()
            .filter(|g| g.tournament_id == Some(p.tournament.id))
        {
            let slot = slots_by_game
                .get(&g.id)
                .context("imported tournament game has no canonical slot")?;
            out.push(BundleRecord::Game {
                game: CanonicalGame::from_legacy(g, slot.key)?,
            })
        }
        if let Some(f) = &p.final_outcome {
            out.push(BundleRecord::FinalOutcome {
                outcome: CanonicalFinalOutcome {
                    tournament_id: p.tournament.id,
                    standings: f.standings.clone(),
                },
            })
        }
    }
    for offer in schedule_projection.offers {
        out.push(BundleRecord::ScheduleOffer { offer });
    }
    sort_records(&mut out);
    Ok((out, schedule_projection.retained_source_rows))
}

#[derive(Clone, Copy)]
struct SchedulableSlot {
    tournament_id: Uuid,
    key: SlotKey,
    white_id: Uuid,
    black_id: Uuid,
}

#[derive(Default)]
struct ScheduleProjection {
    scheduled_at: HashMap<(Uuid, SlotKey), DateTime<Utc>>,
    offers: Vec<CanonicalScheduleOffer>,
    retained_source_rows: usize,
}

fn project_schedules(
    source: &LegacySource,
    audit: &AuditOutcome,
    cutover_at: DateTime<Utc>,
) -> Result<ScheduleProjection> {
    let games = source
        .games
        .iter()
        .map(|game| (game.id, game))
        .collect::<HashMap<_, _>>();
    let mut schedulable_slots = HashMap::new();
    for plan in &audit.plans {
        if plan.tournament.status != "InProgress" {
            continue;
        }
        for slot in &plan.slots {
            if !matches!(slot.clock, Clock::Realtime(_)) {
                continue;
            }
            let SlotPlanStatus::Released { game_id } = slot.status else {
                continue;
            };
            let Some(game) = games.get(&game_id) else {
                continue;
            };
            if game.finished
                || game.turn > 0
                || game.game_status != "NotStarted"
                || game.game_start != "Ready"
            {
                continue;
            }
            schedulable_slots.insert(
                game_id,
                SchedulableSlot {
                    tournament_id: plan.tournament.id,
                    key: slot.key,
                    white_id: slot.white_id,
                    black_id: slot.black_id,
                },
            );
        }
    }

    Ok(project_schedule_rows(
        &source.schedules,
        &schedulable_slots,
        cutover_at,
    ))
}

fn project_schedule_rows(
    schedules: &[LegacyScheduleRow],
    schedulable_slots: &HashMap<Uuid, SchedulableSlot>,
    cutover_at: DateTime<Utc>,
) -> ScheduleProjection {
    let mut grouped = HashMap::<(Uuid, SlotKey), Vec<&LegacyScheduleRow>>::new();
    for schedule in schedules {
        let Some(slot) = schedulable_slots.get(&schedule.game_id) else {
            continue;
        };
        if schedule.tournament_id != slot.tournament_id {
            continue;
        }
        let valid_players = (schedule.proposer_id == slot.white_id
            && schedule.opponent_id == slot.black_id)
            || (schedule.proposer_id == slot.black_id && schedule.opponent_id == slot.white_id);
        if !valid_players {
            continue;
        }
        grouped
            .entry((slot.tournament_id, slot.key))
            .or_default()
            .push(schedule);
    }

    let mut projection = ScheduleProjection::default();
    for (slot, schedules) in grouped {
        let agreed = schedules
            .iter()
            .copied()
            .filter(|schedule| schedule.agreed)
            .collect::<Vec<_>>();
        if agreed.len() == 1 {
            projection.scheduled_at.insert(slot, agreed[0].starts_at);
            projection.retained_source_rows += 1;
            continue;
        }
        if !agreed.is_empty() {
            continue;
        }

        let future = schedules
            .iter()
            .copied()
            .filter(|schedule| schedule.starts_at > cutover_at)
            .collect::<Vec<_>>();
        let proposers = future
            .iter()
            .map(|schedule| schedule.proposer_id)
            .collect::<BTreeSet<_>>();
        if proposers.len() != 1 {
            continue;
        }
        let candidate_times = future
            .iter()
            .map(|schedule| schedule.starts_at)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if !(1..=3).contains(&candidate_times.len()) {
            continue;
        }
        projection.retained_source_rows += candidate_times.len();
        projection.offers.push(CanonicalScheduleOffer {
            tournament_id: slot.0,
            key: slot.1,
            proposer_id: *proposers.first().expect("one proposer was validated"),
            candidate_times,
            // The legacy schema did not record proposal creation time. This is
            // the instant the current offer entered the final-schema system.
            created_at: cutover_at,
        });
    }
    projection
}

fn reconstructed_created_at(
    source: &LegacySource,
    tournament: &LegacyTournamentRow,
) -> DateTime<Utc> {
    let peer_timestamps = [
        tournament.starts_at,
        tournament.ends_at,
        tournament.started_at,
    ]
    .into_iter()
    .flatten();
    let invitation_timestamps = source
        .invitations
        .iter()
        .filter(|invitation| invitation.tournament_id == tournament.id)
        .map(|invitation| invitation.created_at);
    let game_timestamps = source
        .games
        .iter()
        .filter(|game| game.tournament_id == Some(tournament.id))
        .flat_map(|game| [game.created_at, game.updated_at]);
    [tournament.created_at, tournament.updated_at]
        .into_iter()
        .chain(peer_timestamps)
        .chain(invitation_timestamps)
        .chain(game_timestamps)
        .min()
        .expect("the tournament contributes source timestamps")
}

fn kind(r: &BundleRecord) -> &'static str {
    match r {
        BundleRecord::Tournament { .. } => "tournament",
        BundleRecord::Organizer { .. } => "organizer",
        BundleRecord::Membership { .. } => "membership",
        BundleRecord::Invitation { .. } => "invitation",
        BundleRecord::SwissRound { .. } => "swiss_round",
        BundleRecord::Slot { .. } => "slot",
        BundleRecord::Game { .. } => "game",
        BundleRecord::ScheduleOffer { .. } => "schedule_offer",
        BundleRecord::FinalOutcome { .. } => "final_outcome",
    }
}
pub(crate) fn record_counts(rs: &[BundleRecord]) -> BTreeMap<String, u64> {
    let mut x = BTreeMap::new();
    for k in BUNDLE_ORDERING.split(',') {
        x.insert(k.into(), 0);
    }
    for r in rs {
        *x.entry(kind(r).into()).or_default() += 1
    }
    x
}

fn skipped_record_counts(
    source: &LegacySource,
    retained_schedule_rows: usize,
) -> Result<BTreeMap<String, u64>> {
    let source_schedules = u64::try_from(source.schedules.len())
        .context("legacy schedule count does not fit the bundle manifest")?;
    let imported_schedules = u64::try_from(retained_schedule_rows)
        .context("retained legacy schedule count does not fit the bundle manifest")?;
    let skipped_schedules = source_schedules
        .checked_sub(imported_schedules)
        .context("canonical bundle contains more schedules than the legacy source")?;
    Ok(BTreeMap::from([(
        SKIPPED_SCHEDULES_WITHOUT_FINAL_MEANING.into(),
        skipped_schedules,
    )]))
}

fn sort_records(records: &mut [BundleRecord]) {
    records.sort_by_cached_key(|record| {
        let rank = BUNDLE_ORDERING
            .split(',')
            .position(|name| name == kind(record))
            .expect("every canonical record kind is in the ordering contract");
        (
            rank,
            serde_json::to_string(record).expect("canonical record serialization"),
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_tournaments::{audit, double_swiss};
    use chrono::{Duration, TimeZone};
    use tournamint::round_robin::RoundRobinGameId;

    fn round_robin_key(value: usize) -> SlotKey {
        SlotKey::RoundRobin {
            slot: RoundRobinGameId::new(value),
        }
    }

    #[test]
    fn dropped_schedule_metadata_does_not_affect_reconstructed_creation_time() {
        let starts_at = Utc.timestamp_opt(10, 0).unwrap();
        let created_at = Utc.timestamp_opt(5, 0).unwrap();
        let earliest_source_timestamp = Utc.timestamp_opt(1, 0).unwrap();
        let tournament_id = Uuid::from_u128(1);
        let member_id = Uuid::from_u128(2);
        let tournament = LegacyTournamentRow {
            id: tournament_id,
            nanoid: String::from("legacy"),
            name: String::from("Legacy"),
            description: String::new(),
            scoring: String::from("Game"),
            tiebreaker: vec![Some(String::from("RawPoints"))],
            seats: 4,
            min_seats: 2,
            rounds: 1,
            invite_only: false,
            mode: String::from("DoubleRoundRobin"),
            time_mode: String::from("Real Time"),
            time_base: Some(180),
            time_increment: Some(2),
            band_upper: None,
            band_lower: None,
            start_mode: String::from("Date"),
            starts_at: Some(starts_at),
            ends_at: None,
            started_at: None,
            round_duration: None,
            status: String::from("NotStarted"),
            created_at,
            updated_at: Utc.timestamp_opt(3, 0).unwrap(),
            series: None,
        };
        let source = LegacySource {
            metadata: LegacySourceMetadata {
                database_name: String::from("hive-test"),
            },
            tournaments: vec![tournament],
            memberships: vec![LegacyMembershipRow {
                tournament_id,
                user_id: member_id,
            }],
            organizers: Vec::new(),
            invitations: vec![LegacyInvitationRow {
                tournament_id,
                invitee_id: member_id,
                created_at: earliest_source_timestamp,
                declined_at: None,
            }],
            games: Vec::new(),
            schedules: vec![LegacyScheduleRow {
                id: Uuid::from_u128(3),
                game_id: Uuid::from_u128(4),
                tournament_id,
                proposer_id: member_id,
                opponent_id: Uuid::from_u128(5),
                starts_at: Utc.timestamp_opt(0, 0).unwrap(),
                agreed: true,
                notified: false,
            }],
            series: Vec::new(),
            series_organizers: Vec::new(),
        };
        let audit = audit::run(&source);
        assert!(!audit.has_hard_failures());
        let bundle = EncodedBundle::build(&source, &audit).unwrap();
        assert_eq!(
            bundle.manifest.skipped_record_counts[SKIPPED_SCHEDULES_WITHOUT_FINAL_MEANING],
            1
        );
        let records = validate(&bundle.jsonl, &bundle.manifest).unwrap();
        let tournament = records
            .iter()
            .find_map(|record| match record {
                BundleRecord::Tournament { tournament } => Some(tournament.as_ref()),
                _ => None,
            })
            .expect("canonical Tournament record");
        let membership = records
            .iter()
            .find_map(|record| match record {
                BundleRecord::Membership { membership } => Some(membership),
                _ => None,
            })
            .expect("canonical Membership record");

        assert_eq!(tournament.id, tournament_id);
        assert_eq!(tournament.starts_at, Some(starts_at));
        assert_eq!(tournament.description, Some(String::new()));
        assert_eq!(tournament.created_at, earliest_source_timestamp);
        assert_eq!(membership.accepted_at, earliest_source_timestamp);
    }

    #[test]
    fn finished_double_swiss_bundle_preserves_recovered_accepted_ratings() {
        let (tournament, memberships, games) = double_swiss::tests::valid_fixture();
        let source = LegacySource {
            metadata: LegacySourceMetadata {
                database_name: String::from("hive-test"),
            },
            tournaments: vec![tournament],
            memberships,
            organizers: Vec::new(),
            invitations: Vec::new(),
            games,
            schedules: Vec::new(),
            series: Vec::new(),
            series_organizers: Vec::new(),
        };
        let audit = audit::run(&source);
        assert!(!audit.has_hard_failures());

        let bundle = EncodedBundle::build(&source, &audit).unwrap();
        let records = validate(&bundle.jsonl, &bundle.manifest).unwrap();
        let rounds = records
            .iter()
            .filter_map(|record| match record {
                BundleRecord::SwissRound { round } => Some(round),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(rounds.len(), 2);
        let expected = (0..4)
            .map(|index| CanonicalAcceptedRating {
                player: PlayerId::new(index),
                rating: 1_501 + u32::try_from(index).unwrap(),
            })
            .collect::<Vec<_>>();
        assert!(rounds
            .iter()
            .all(|round| round.accepted_ratings == expected));
    }

    #[test]
    fn unique_legacy_agreement_becomes_the_slot_schedule_and_drops_siblings() {
        let cutover_at = Utc.timestamp_opt(10, 0).unwrap();
        let tournament_id = Uuid::from_u128(1);
        let game_id = Uuid::from_u128(2);
        let white_id = Uuid::from_u128(3);
        let black_id = Uuid::from_u128(4);
        let key = round_robin_key(7);
        let slots = HashMap::from([(
            game_id,
            SchedulableSlot {
                tournament_id,
                key,
                white_id,
                black_id,
            },
        )]);
        let agreed_at = cutover_at - Duration::hours(1);
        let schedules = vec![
            LegacyScheduleRow {
                id: Uuid::from_u128(5),
                game_id,
                tournament_id,
                proposer_id: white_id,
                opponent_id: black_id,
                starts_at: agreed_at,
                agreed: true,
                notified: false,
            },
            LegacyScheduleRow {
                id: Uuid::from_u128(6),
                game_id,
                tournament_id,
                proposer_id: black_id,
                opponent_id: white_id,
                starts_at: cutover_at + Duration::hours(1),
                agreed: false,
                notified: true,
            },
        ];

        let projection = project_schedule_rows(&schedules, &slots, cutover_at);
        assert_eq!(projection.scheduled_at[&(tournament_id, key)], agreed_at);
        assert!(projection.offers.is_empty());
        assert_eq!(projection.retained_source_rows, 1);

        let mut conflicting = schedules;
        conflicting[1].agreed = true;
        let projection = project_schedule_rows(&conflicting, &slots, cutover_at);
        assert!(projection.scheduled_at.is_empty());
        assert!(projection.offers.is_empty());
        assert_eq!(projection.retained_source_rows, 0);
    }

    #[test]
    fn only_unambiguous_unique_future_candidates_become_an_offer() {
        let cutover_at = Utc.timestamp_opt(10, 0).unwrap();
        let tournament_id = Uuid::from_u128(1);
        let game_id = Uuid::from_u128(2);
        let ambiguous_game_id = Uuid::from_u128(3);
        let white_id = Uuid::from_u128(4);
        let black_id = Uuid::from_u128(5);
        let key = round_robin_key(7);
        let ambiguous_key = round_robin_key(8);
        let slots = HashMap::from([
            (
                game_id,
                SchedulableSlot {
                    tournament_id,
                    key,
                    white_id,
                    black_id,
                },
            ),
            (
                ambiguous_game_id,
                SchedulableSlot {
                    tournament_id,
                    key: ambiguous_key,
                    white_id,
                    black_id,
                },
            ),
        ]);
        let schedule = |id, game_id, proposer_id, opponent_id, starts_at| LegacyScheduleRow {
            id: Uuid::from_u128(id),
            game_id,
            tournament_id,
            proposer_id,
            opponent_id,
            starts_at,
            agreed: false,
            notified: false,
        };
        let first = cutover_at + Duration::hours(1);
        let second = cutover_at + Duration::hours(2);
        let third = cutover_at + Duration::hours(3);
        let schedules = vec![
            schedule(10, game_id, white_id, black_id, first),
            schedule(11, game_id, white_id, black_id, second),
            schedule(12, game_id, white_id, black_id, second),
            schedule(13, game_id, white_id, black_id, third),
            schedule(
                14,
                game_id,
                white_id,
                black_id,
                cutover_at - Duration::seconds(1),
            ),
            schedule(15, ambiguous_game_id, white_id, black_id, first),
            schedule(16, ambiguous_game_id, black_id, white_id, second),
        ];

        let projection = project_schedule_rows(&schedules, &slots, cutover_at);
        assert!(projection.scheduled_at.is_empty());
        assert_eq!(projection.offers.len(), 1);
        assert_eq!(projection.offers[0].key, key);
        assert_eq!(
            projection.offers[0].candidate_times,
            vec![first, second, third]
        );
        assert_eq!(projection.retained_source_rows, 3);

        let over_capacity = vec![
            schedule(20, game_id, white_id, black_id, first),
            schedule(21, game_id, white_id, black_id, second),
            schedule(22, game_id, white_id, black_id, third),
            schedule(
                23,
                game_id,
                white_id,
                black_id,
                cutover_at + Duration::hours(4),
            ),
        ];
        let projection = project_schedule_rows(&over_capacity, &slots, cutover_at);
        assert!(projection.offers.is_empty());
        assert_eq!(projection.retained_source_rows, 0);
    }
}
