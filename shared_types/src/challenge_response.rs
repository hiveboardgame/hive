use crate::{ChallengeId, ChallengeVisibility, GameSpeed, TimeMode, UserResponse};
use chrono::prelude::*;
use hive_lib::ColorChoice;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChallengeResponse {
    pub id: Uuid,
    pub challenge_id: ChallengeId,
    pub challenger: UserResponse,
    pub opponent: Option<UserResponse>,
    pub game_type: String,
    pub rated: bool,
    pub visibility: ChallengeVisibility,
    pub color_choice: ColorChoice,
    pub created_at: DateTime<Utc>,
    pub challenger_rating: u64,
    pub time_mode: TimeMode,         // Correspondence, Timed, Untimed
    pub time_base: Option<i32>,      // Secons
    pub time_increment: Option<i32>, // Seconds
    pub speed: GameSpeed,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
}
