use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]

pub struct RatingHistoryResponse {
    pub rating: u64,
    pub updated_at: DateTime<Utc>,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{Game},
    DbConn,
};
use shared_types::GameSpeed;
use uuid::Uuid;
use anyhow::Result;
impl RatingHistoryResponse {
    pub async fn get_rating_history_from_uuid_and_speed(
        id: &Uuid,
        game_speed: &GameSpeed,
        conn: &mut DbConn<'_>
    ) -> Result<Vec<Self>> {
        let games = Game::get_rating_history_for_player(*id, game_speed, conn).await?;
        Ok(
            games
                .into_iter()
                .map(|game| RatingHistoryResponse {
                    rating: game.rating.floor() as u64,
                    updated_at: game.updated_at,
                })
                .collect::<Vec<_>>(),
        )
    }
}
}}
