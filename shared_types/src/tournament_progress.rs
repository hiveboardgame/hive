use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum SlotAdminAction {
    RecordResult,
    ReplaceResult,
    ClearResult,
    SetDeadline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SwissProgress {
    AwaitingResults,
    ReadyForNextRound,
    PairingExhausted,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SwissRoundSummary {
    pub round: u32,
    pub resolved_encounters: u32,
    pub total_encounters: u32,
}
