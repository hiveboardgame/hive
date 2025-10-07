use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::GameSpeed;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RatingHistoryResponse {
    pub data: Vec<RatingHistoryEntry>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RatingHistoryEntry {
    pub speed: GameSpeed,
    pub rating: u64,
    pub updated_at: DateTime<Utc>,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    use anyhow::Result;
    use db_lib::{models::Rating, DbConn};
    use std::str::FromStr;
    use uuid::Uuid;
    impl RatingHistoryResponse {
        pub async fn from_uuid(id: &Uuid, game_speed: &GameSpeed, conn: &mut DbConn<'_>) -> Result<Self> {
            let ratings = Rating::history_for_uuid(id, game_speed, conn).await?;
            // println!("RatingHistoryResponse::from_uuid fetched some ratings");
            Ok(Self::from_ratings(ratings))
        }

        pub fn from_ratings(ratings: Vec<Rating>) -> Self {
            let mut data = ratings
                .into_iter()
                .map(|rating| RatingHistoryEntry {
                    speed: GameSpeed::from_str(&rating.speed).expect("Rating to have a valid GameSpeed"),
                    rating: rating.rating.floor() as u64,
                    updated_at: rating.updated_at,
                })
                .collect::<Vec<_>>();
            data.sort_by_key(|entry| entry.updated_at);
            // println!("Converted ratings into RatingHistoryResponse with {} entries", data.len());
            Self { data }
        }
    }
}}
