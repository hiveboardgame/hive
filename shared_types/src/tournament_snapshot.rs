use serde::{
    de::{DeserializeOwned, Error as DeError},
    Deserialize,
    Deserializer,
    Serialize,
};
use serde_json::Value as JsonValue;
use tournamint::{elimination::EliminationNodeId, standings::CriterionValue, Score};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Value {
    Score(Score),
    Integer(u64),
    SignedInteger(i64),
    NotApplicable,
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Kind {
            Score,
            Integer,
            SignedInteger,
            NotApplicable,
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireValue {
            kind: Kind,
            value: Option<JsonValue>,
        }

        let WireValue { kind, value } = WireValue::deserialize(deserializer)?;
        match kind {
            Kind::Score => with_json_content(value, "value", Self::Score),
            Kind::Integer => with_json_content(value, "value", Self::Integer),
            Kind::SignedInteger => with_json_content(value, "value", Self::SignedInteger),
            Kind::NotApplicable => match value {
                None | Some(JsonValue::Null) => Ok(Self::NotApplicable),
                Some(_) => Err(DeError::custom(
                    "unexpected value for not-applicable standing",
                )),
            },
        }
    }
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

fn with_json_content<T, U, E>(
    content: Option<JsonValue>,
    field: &'static str,
    into_value: impl FnOnce(T) -> U,
) -> Result<U, E>
where
    T: DeserializeOwned,
    E: DeError,
{
    let content = content.ok_or_else(|| E::missing_field(field))?;
    serde_json::from_value(content)
        .map(into_value)
        .map_err(E::custom)
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
