use serde::{Deserialize, Serialize};
use shared_types::GameId;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct HeartbeatResponse {
    pub game_id: GameId,
    pub black_time_left: Duration,
    pub white_time_left: Duration,
}
