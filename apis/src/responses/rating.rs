use serde::{Deserialize, Serialize};
use shared_types::{Certainty, GameSpeed};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RatingResponse {
    pub speed: GameSpeed,
    pub rating: u64,
    pub played: i64,
    pub win: i64,
    pub loss: i64,
    pub draw: i64,
    pub certainty: Certainty,
    pub user_uid: Uuid,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::Rating,
};
use std::str::FromStr;
impl RatingResponse {
    pub fn from_rating(rating: &Rating) -> Self {
        Self {
            speed: GameSpeed::from_str(&rating.speed).expect("Rating to have a valid GameSpeed"),
            rating: rating.rating.floor() as u64,
            played: rating.played,
            win: rating.won,
            loss: rating.lost,
            draw: rating.draw,
            certainty: Certainty::from_deviation(rating.deviation),
            user_uid: rating.user_uid,
        }
    }
}
}}
