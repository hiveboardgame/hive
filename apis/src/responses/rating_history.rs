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
    pub async fn get_rating_history_from_uuid_and_speed(id: &Uuid, game_speed: &GameSpeed, conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
        let games = Game::get_rated_games_for_player_and_speed_sorted(*id, game_speed, conn).await?;
        Ok(
            games
                .into_iter()
                .map(|g| {
                    let (r, dc, ts) = if g.white_id == *id {
                        (
                            g.white_rating.expect("missing white_rating"),
                            g.white_rating_change.expect("missing white_rating_change"),
                            g.updated_at,
                        )
                    } else if g.black_id == *id {
                        (
                            g.black_rating.expect("missing black_rating"),
                            g.black_rating_change.expect("missing black_rating_change"),
                            g.updated_at,
                        )
                    } else {
                        unreachable!("Game does not match the given player ID");
                    };
                    RatingHistoryResponse {
                        rating: (r + dc).floor() as u64,
                        updated_at: ts,
                    }
                })
                .collect::<Vec<_>>(),
        )
    }
}
}}