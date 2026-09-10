use anyhow::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::{
    tournament::{standings::Snapshot, Clock, Config, GameOutcome, Resolution, SlotKey},
    TournamentStatus,
};
use std::{collections::BTreeMap, str::FromStr};
use tournamint::{swiss::RoundPairings, PlayerId};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalTournament {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: Option<String>,
    pub seats: i32,
    pub min_seats: i32,
    pub invite_only: bool,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
    pub configuration: Config,
    pub lifecycle: TournamentStatus,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalMembership {
    pub tournament_id: Uuid,
    pub user_id: Uuid,
    pub accepted_at: DateTime<Utc>,
    pub pairing_number: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSlot {
    pub tournament_id: Uuid,
    pub key: SlotKey,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub clock: Clock,
    pub resolution: Option<Resolution>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub deadline_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSwissRound {
    pub tournament_id: Uuid,
    pub round_id: i64,
    pub pairings: RoundPairings,
    pub accepted_ratings: Vec<CanonicalAcceptedRating>,
    pub accepted_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalAcceptedRating {
    pub player: PlayerId,
    pub rating: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalFinalOutcome {
    pub tournament_id: Uuid,
    pub standings: Snapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyTournamentStatus {
    NotStarted,
    InProgress,
    Finished,
}

impl FromStr for LegacyTournamentStatus {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "NotStarted" => Ok(Self::NotStarted),
            "InProgress" => Ok(Self::InProgress),
            "Finished" => Ok(Self::Finished),
            _ => anyhow::bail!("unsupported legacy tournament status {value:?}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacySource {
    pub metadata: LegacySourceMetadata,
    pub tournaments: Vec<LegacyTournamentRow>,
    pub memberships: Vec<LegacyMembershipRow>,
    pub organizers: Vec<LegacyOrganizerRow>,
    pub invitations: Vec<LegacyInvitationRow>,
    pub games: Vec<LegacyGameRow>,
    pub schedules: Vec<LegacyScheduleRow>,
    pub series: Vec<LegacySeriesRow>,
    pub series_organizers: Vec<LegacySeriesOrganizerRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacySourceMetadata {
    pub database_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyTournamentRow {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub seats: i32,
    pub min_seats: i32,
    pub rounds: i32,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_mode: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyMembershipRow {
    pub tournament_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyOrganizerRow {
    pub tournament_id: Uuid,
    pub organizer_id: Uuid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyInvitationRow {
    pub tournament_id: Uuid,
    pub invitee_id: Uuid,
    pub created_at: DateTime<Utc>,
    // The actual pre-final schema has no declined column. Keeping the target
    // shape explicit prevents an importer from inventing historical state.
    pub declined_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyGameRow {
    pub id: Uuid,
    pub nanoid: String,
    pub current_player_id: Uuid,
    pub black_id: Uuid,
    pub finished: bool,
    pub game_status: String,
    pub history: String,
    pub game_control_history: String,
    pub turn: i32,
    pub white_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub hashes: Vec<Option<i64>>,
    pub conclusion: String,
    pub tournament_id: Option<Uuid>,
    pub tournament_game_result: String,
    pub game_start: String,
    pub move_times: Vec<Option<i64>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyScheduleRow {
    pub id: Uuid,
    pub game_id: Uuid,
    pub tournament_id: Uuid,
    pub proposer_id: Uuid,
    pub opponent_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub agreed: bool,
    pub notified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacySeriesRow {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacySeriesOrganizerRow {
    pub series_id: Uuid,
    pub organizer_id: Uuid,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportedLegacyMapping {
    RoundRobin,
    FinishedDoubleSwiss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participant {
    pub user_id: Uuid,
    /// Stable one-based tournament pairing number.
    pub pairing_number: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roster {
    pub participants: Vec<Participant>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TournamentPlan {
    pub tournament: LegacyTournamentRow,
    pub mapping: SupportedLegacyMapping,
    pub configuration: Config,
    pub roster: Option<Roster>,
    pub slots: Vec<SlotPlan>,
    pub accepted_swiss_rounds: Vec<AcceptedSwissRoundPlan>,
    pub expected_rank_groups: Vec<LegacyRankGroup>,
    pub final_outcome: Option<FinalOutcomePlan>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotPlan {
    pub key: SlotKey,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub clock: Clock,
    pub status: SlotPlanStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum SlotPlanStatus {
    Planned,
    Released { game_id: Uuid },
    Sealed { game_id: Uuid, outcome: GameOutcome },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedSwissRoundPlan {
    pub round_index: u32,
    pub pairings: Vec<SwissPairingPlan>,
    pub accepted_ratings: Vec<AcceptedRatingPlan>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedRatingPlan {
    pub user_id: Uuid,
    pub rating: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwissPairingPlan {
    pub white_id: Uuid,
    pub black_id: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalOutcomePlan {
    pub standings: Snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyRankGroup {
    pub competition_rank: u32,
    pub user_ids: Vec<Uuid>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Warning,
    HardFailure,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditDiagnostic {
    pub severity: AuditSeverity,
    pub code: String,
    pub tournament_id: Option<Uuid>,
    pub message: String,
}

impl AuditDiagnostic {
    pub fn warning(tournament_id: Option<Uuid>, code: &str, message: impl Into<String>) -> Self {
        Self {
            severity: AuditSeverity::Warning,
            code: code.to_string(),
            tournament_id,
            message: message.into(),
        }
    }

    pub fn hard_failure(
        tournament_id: Option<Uuid>,
        code: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: AuditSeverity::HardFailure,
            code: code.to_string(),
            tournament_id,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TournamentAuditReport {
    pub tournament_id: Uuid,
    pub legacy_mode: String,
    pub legacy_status: String,
    pub legacy_configuration: LegacyConfigurationReport,
    pub intended_mapping: Option<SupportedLegacyMapping>,
    pub mapped_starts_at: Option<DateTime<Utc>>,
    pub mapped_configuration: Option<Config>,
    pub participants: u64,
    pub organizers: u64,
    pub invitations: u64,
    pub declined_invitations: u64,
    pub series: Option<Uuid>,
    pub games: u64,
    pub schedules: u64,
    pub rounds: u64,
    pub pairings: u64,
    pub byes_or_sit_outs: u64,
    pub recognized_committee_results: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyConfigurationReport {
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub seats: i32,
    pub min_seats: i32,
    pub rounds: i32,
    pub invite_only: bool,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_mode: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReport {
    pub source: LegacySourceMetadata,
    pub tournaments: Vec<TournamentAuditReport>,
    pub counts: BTreeMap<String, u64>,
    pub diagnostics: Vec<AuditDiagnostic>,
}

impl AuditReport {
    pub fn has_hard_failures(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == AuditSeverity::HardFailure)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditOutcome {
    pub report: AuditReport,
    pub plans: Vec<TournamentPlan>,
}

impl AuditOutcome {
    pub fn has_hard_failures(&self) -> bool {
        self.report.has_hard_failures()
    }
}
