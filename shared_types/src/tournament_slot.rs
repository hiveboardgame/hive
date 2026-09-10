use crate::Clock;
use serde::{Deserialize, Serialize};
use tournamint::{
    elimination::EliminationNodeId,
    round_robin::RoundRobinGameId,
    series::SeriesGameId,
    swiss::SwissGameId,
    GameOutcome,
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case", deny_unknown_fields)]
pub enum SlotKey {
    RoundRobin {
        slot: RoundRobinGameId,
    },
    Swiss {
        slot: SwissGameId,
    },
    Elimination {
        node: EliminationNodeId,
        slot: SeriesGameId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "outcome",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Resolution {
    Result(GameOutcome),
    Withdrawal(GameOutcome),
    Clinched,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: Uuid,
    pub key: SlotKey,
    pub white: Uuid,
    pub black: Uuid,
    pub clock: Clock,
    pub resolution: Option<Resolution>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tournamint::swiss::SwissLeg;

    #[test]
    fn slot_key_json_is_stable() {
        let cases = [
            (
                SlotKey::RoundRobin {
                    slot: RoundRobinGameId::new(7),
                },
                json!({"format": "round_robin", "slot": 7}),
            ),
            (
                SlotKey::Swiss {
                    slot: SwissGameId {
                        round_index: 2,
                        pairing_index: 3,
                        leg: SwissLeg::Second,
                    },
                },
                json!({
                    "format": "swiss",
                    "slot": {
                        "round_index": 2,
                        "pairing_index": 3,
                        "leg": "second"
                    }
                }),
            ),
            (
                SlotKey::Elimination {
                    node: EliminationNodeId::new(5),
                    slot: SeriesGameId::new(8),
                },
                json!({"format": "elimination", "node": 5, "slot": 8}),
            ),
        ];

        for (key, json) in cases {
            assert_eq!(serde_json::to_value(key).unwrap(), json);
            assert_eq!(serde_json::from_value::<SlotKey>(json).unwrap(), key);
        }
    }
}
