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
    models::{Rating, User},
    DbConn,
};
use std::str::FromStr;
use anyhow::Result;
impl RatingResponse {
    pub async fn from_uuid(id: &Uuid, game_speed: &GameSpeed, conn: &mut DbConn<'_>) -> Result<Self> {
        let rating = Rating::for_uuid(id, game_speed, conn).await?;
        Ok(Self::from_rating(&rating))
    }

    pub async fn from_user(user: &User, game_speed: &GameSpeed, conn: &mut DbConn<'_>) -> Result<Self> {
        let rating = Rating::for_uuid(&user.id, game_speed, conn).await?;
        Ok(Self::from_rating(&rating))
    }

    pub async fn from_username(username: &str, game_speed: &GameSpeed, conn: &mut DbConn<'_>) -> Result<Self> {
        let user = User::find_by_username(username, conn).await?;
        let rating = Rating::for_uuid(&user.id, game_speed, conn).await?;
        Ok(Self::from_rating(&rating))
    }

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
