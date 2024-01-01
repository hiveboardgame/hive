use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct UserResponse {
    pub username: String,
    pub uid: Uuid,
    pub rating: u64,
    pub played: i64,
    pub win: i64,
    pub loss: i64,
    pub draw: i64,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{rating::Rating, user::User},
    DbPool,
};
use anyhow::Result;
impl UserResponse {
    pub async fn from_uuid(id: &Uuid, pool: &DbPool) -> Result<Self> {
        let user = User::find_by_uuid(id, pool).await?;
        let rating = Rating::for_uuid(id, pool).await?;

        Ok(Self::from_user_and_rating(&user, &rating))
    }

    pub async fn from_username(username: &str, pool: &DbPool) -> Result<Self> {
        let user = User::find_by_username(username, pool).await?;
        let rating = Rating::for_uuid(&user.id, pool).await?;

        Ok(Self::from_user_and_rating(&user, &rating))
    }

    pub fn from_user_and_rating(user:&User, rating:&Rating) -> Self {
        Self {
            username: user.username.clone(),
            uid: user.id,
            rating: rating.rating.floor() as u64,
            played: rating.played,
            win: rating.won,
            loss: rating.lost,
            draw: rating.draw,
        }
    }
}
}}
