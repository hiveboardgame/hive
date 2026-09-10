use serde::{Deserialize, Serialize};
use tournamint::{elimination::EliminationNodeId, standings::CriterionValue, Score};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Value {
    Score(Score),
    Integer(u64),
    SignedInteger(i64),
    NotApplicable,
}

impl From<CriterionValue> for Value {
    fn from(value: CriterionValue) -> Self {
        match value {
            CriterionValue::Score(score) => Self::Score(score),
            CriterionValue::Integer(value) => Self::Integer(value),
            CriterionValue::NotApplicable => Self::NotApplicable,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub user_id: Uuid,
    pub primary_score: Value,
    /// Logical games that actually began. Administrative awards still affect
    /// score and result totals without pretending that a game was played.
    pub games_played: u32,
    pub matches_played: Option<u32>,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    /// Format-defined auxiliary counts in their canonical order.
    pub counts: Vec<u32>,
    /// Values aligned with the standings criteria in tournament configuration.
    pub values: Vec<Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "place",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Placement {
    CompetitionRank(u32),
    EliminationTier(u32),
}

impl Placement {
    pub const fn rank(self) -> u32 {
        match self {
            Self::CompetitionRank(rank) | Self::EliminationTier(rank) => rank,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub placement: Placement,
    pub rows: Vec<Row>,
    /// Zero-based configured criterion that first separated this group from
    /// the preceding group. The leading group has no preceding boundary.
    #[serde(default)]
    pub separated_by: Option<u32>,
}

/// Tournament standings with the schema stored once and row values aligned by
/// position. The owning tournament supplies identity, format, and lifecycle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub groups: Vec<Group>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum Source {
    InitialSeed { seed_index: u32 },
    WinnerOf { node_id: EliminationNodeId },
    LoserOf { node_id: EliminationNodeId },
    Vacant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "resolution", rename_all = "snake_case", deny_unknown_fields)]
pub enum Resolution {
    PlayedWinner { winner: Uuid, loser: Uuid },
    AutomaticAdvance { player: Uuid },
    Walkover { winner: Uuid, withdrawn: Uuid },
    Vacancy,
    SkippedConditionalBranch,
}

#[cfg(test)]
mod tests {
    use super::{Group, Placement, Row, Snapshot, Value};
    use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
    use tournamint::Score;
    use uuid::Uuid;

    fn standings_with_all_value_kinds() -> Snapshot {
        Snapshot {
            groups: vec![Group {
                placement: Placement::CompetitionRank(1),
                rows: vec![Row {
                    user_id: Uuid::nil(),
                    primary_score: Value::Score(Score::new(u32::MAX)),
                    games_played: 0,
                    matches_played: None,
                    wins: 0,
                    draws: 0,
                    losses: 0,
                    counts: vec![],
                    values: vec![
                        Value::Integer(u64::MAX),
                        Value::SignedInteger(i64::MIN),
                        Value::SignedInteger(i64::MAX),
                        Value::NotApplicable,
                    ],
                }],
                separated_by: None,
            }],
        }
    }

    #[test]
    fn standings_with_not_applicable_values_round_trip_through_the_websocket_codec() {
        let standings = standings_with_all_value_kinds();
        let bytes = MsgpackSerdeCodec::encode(&standings).unwrap();
        let decoded: Snapshot = MsgpackSerdeCodec::decode(&bytes).unwrap();
        assert_eq!(decoded, standings);
    }

    #[test]
    fn standings_preserve_integer_bounds_through_json_storage() {
        let standings = standings_with_all_value_kinds();
        let value = serde_json::to_value(&standings).unwrap();
        let text = serde_json::to_string(&value).unwrap();
        let stored: serde_json::Value = serde_json::from_str(&text).unwrap();
        let decoded: Snapshot = serde_json::from_value(stored).unwrap();
        assert_eq!(decoded, standings);
    }
}
